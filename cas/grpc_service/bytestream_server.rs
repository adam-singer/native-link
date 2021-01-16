// Copyright 2020 Nathan (Blaise) Bruer.  All rights reserved.

use std::convert::TryFrom;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use async_fixed_buffer::AsyncFixedBuf;
use drop_guard::DropGuard;
use futures::{stream::unfold, Stream};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tonic::{Request, Response, Status, Streaming};

use proto::google::bytestream::{
    byte_stream_server::ByteStream, byte_stream_server::ByteStreamServer as Server, QueryWriteStatusRequest,
    QueryWriteStatusResponse, ReadRequest, ReadResponse, WriteRequest, WriteResponse,
};

use common::{log, DigestInfo};
use error::{error_if, Error, ResultExt};
use store::Store;

pub struct ByteStreamServer {
    store: Arc<dyn Store>,
    // Buffer size for transferring data between grpc endpoint and store.
    write_buffer_stream_size: usize,
    // Buffer size for transferring data between store and grpc endpoint.
    read_buffer_stream_size: usize,
    // Max number of bytes to send on each grpc stream chunk.
    max_bytes_per_stream: usize,
}

struct ReaderState {
    max_bytes_per_stream: usize,
    was_shutdown: bool,
    stream_closer: Box<dyn FnMut() + Sync + Send>,
    rx: Box<dyn AsyncRead + Sync + Send + Unpin>,
    reading_future: Box<tokio::task::JoinHandle<Result<(), Error>>>,
}

impl ReaderState {
    async fn shutdown(&mut self) -> Result<(), Error> {
        // let mut me = Pin::new(&mut self);
        self.was_shutdown = true;
        // Close stream then wait for reader stream to finish.
        (self.stream_closer)();
        let mut drainer = tokio::io::sink();
        // Note: We ignore errors here or else we will get "Sender Disconnected" errors.
        let _ = tokio::io::copy(&mut self.rx, &mut drainer).await;

        let reading_future = Pin::new(&mut self.reading_future);
        reading_future
            .await
            .err_tip(|| "Could not join result from store thread")
            .and_then(|r| r)?; // Translates Result<Result<T, E>, E> -> Result<T, E>.
        Ok(())
    }
}

type ReadStream = Pin<Box<dyn Stream<Item = Result<ReadResponse, Status>> + Send + Sync + 'static>>;

impl ByteStreamServer {
    pub fn new(store: Arc<dyn Store>) -> Self {
        ByteStreamServer {
            store: store,
            // TODO(allada) Make this configurable.
            // This value was choosen only because it is a common mem page size.
            write_buffer_stream_size: 2 << 20, // 2Mb.
            read_buffer_stream_size: 2 << 20,  // 2Mb.
            // According to https://github.com/grpc/grpc.github.io/issues/371 16KiB - 64KiB is optimal.
            max_bytes_per_stream: 2 << 15, // 64Kb.
        }
    }

    pub fn into_service(self) -> Server<ByteStreamServer> {
        Server::new(self)
    }

    async fn inner_read(&self, grpc_request: Request<ReadRequest>) -> Result<Response<ReadStream>, Error> {
        let read_request = grpc_request.into_inner();

        let read_limit =
            usize::try_from(read_request.read_limit).err_tip(|| "read_limit has is not convertable to usize")?;
        let resource_info = ResourceInfo::new(&read_request.resource_name)?;
        let digest = DigestInfo::try_new(&resource_info.hash, resource_info.expected_size)?;

        let mut raw_fixed_buffer = AsyncFixedBuf::new(vec![0u8; self.read_buffer_stream_size].into_boxed_slice());
        let stream_closer = raw_fixed_buffer.get_closer();
        let (rx, mut tx) = tokio::io::split(raw_fixed_buffer);
        let rx: Box<dyn tokio::io::AsyncRead + Sync + Send + Unpin> = if read_limit != 0 {
            Box::new(rx.take(u64::try_from(read_limit).err_tip(|| "read_limit has is not convertable to u64")?))
        } else {
            Box::new(rx)
        };

        let store_clone = self.store.clone();
        let reading_future = Box::new(tokio::spawn(async move {
            let store = Pin::new(store_clone.as_ref());
            let read_limit = if read_limit != 0 { Some(read_limit) } else { None };
            let p = store
                .get_part(digest, &mut tx, read_request.read_offset as usize, read_limit)
                .await
                .err_tip(|| "Error retrieving data from store");
            p
        }));

        // This allows us to call a destructor when the the object is dropped.
        let state = Some(DropGuard::new(
            ReaderState {
                stream_closer: stream_closer,
                rx: rx,
                max_bytes_per_stream: self.max_bytes_per_stream,
                reading_future: reading_future,
                was_shutdown: false,
            },
            |mut me| {
                // This is the hot path. It is only under rare circumstances
                // that we would not be shutdown already. This lambda should
                // only be triggered in the event the stream abruptly closes.
                if me.was_shutdown {
                    return;
                }
                // Failing to launch it into it's own spawn would result in
                // a thread being blocked.
                tokio::spawn(async move {
                    // Ignoring errors because we don't have anywhere to publish them.
                    let _ = me.shutdown().await;
                });
            },
        ));

        Ok(Response::new(Box::pin(unfold(state, move |state| async {
            let mut state = state?; // If state is None, we have already sent error if needed (None is Done).
            let mut response = ReadResponse {
                data: vec![0u8; state.max_bytes_per_stream],
            };
            let read_result = state.rx.read(&mut response.data[..]).await;
            match read_result.err_tip(|| "Error reading data from underlying store") {
                Ok(sz) => {
                    response.data.resize(sz, 0u8);
                    // Receiving zero bytes is an EOF.
                    if sz == 0 {
                        // We want to gracefully shutdown and in the event there
                        // are any errors present them here.
                        return state
                            .shutdown()
                            .await
                            .map_or_else(|e| Some((Err(e.into()), None)), |_| None);
                    }
                    Some((Ok(response), Some(state)))
                }
                Err(e) => Some((Err(e.into()), None)),
            }
        }))))
    }

    async fn inner_write(
        &self,
        grpc_request: Request<Streaming<WriteRequest>>,
    ) -> Result<Response<WriteResponse>, Error> {
        let mut stream = WriteRequestStreamWrapper::from(grpc_request.into_inner())
            .await
            .err_tip(|| "Could not unwrap first stream message")?;

        let raw_buffer = vec![0u8; self.write_buffer_stream_size].into_boxed_slice();
        let (rx, mut tx) = tokio::io::split(AsyncFixedBuf::new(raw_buffer));

        let join_handle = {
            let store_clone = self.store.clone();
            let hash = stream.hash.clone();
            let expected_size = stream.expected_size;
            tokio::spawn(async move {
                let rx = Box::new(rx.take(expected_size as u64));
                let store = Pin::new(store_clone.as_ref());
                store.update(DigestInfo::try_new(&hash, expected_size)?, rx).await
            })
        };

        while let Some(write_request) = stream.next().await.err_tip(|| "Stream closed early")? {
            tx.write_all(&write_request.data)
                .await
                .err_tip(|| "Error writing to store stream")?;
        }
        join_handle
            .await
            .err_tip(|| "Error joining promise")?
            .err_tip(|| "Error updating inner store")?;
        Ok(Response::new(WriteResponse {
            committed_size: stream.bytes_received as i64,
        }))
    }
}

struct ResourceInfo<'a> {
    // TODO(allada) We do not support instance naming yet.
    _instance_name: &'a str,
    // TODO(allada) Currently we do not support stream resuming, this is
    // the field we would need.
    _uuid: Option<&'a str>,
    hash: &'a str,
    expected_size: usize,
}

impl<'a> ResourceInfo<'a> {
    fn new(resource_name: &'a str) -> Result<ResourceInfo<'a>, Error> {
        let mut parts = resource_name.splitn(6, '/');
        const ERROR_MSG: &str = concat!(
            "Expected resource_name to be of pattern ",
            "'{instance_name}/uploads/{uuid}/blobs/{hash}/{size}' or ",
            "'{instance_name}/blobs/{hash}/{size}'",
        );
        let instance_name = &parts.next().err_tip(|| ERROR_MSG)?;
        let mut blobs_or_uploads: &str = parts.next().err_tip(|| ERROR_MSG)?;
        let mut uuid = None;
        if &blobs_or_uploads == &"uploads" {
            uuid = Some(parts.next().err_tip(|| ERROR_MSG)?);
            blobs_or_uploads = parts.next().err_tip(|| ERROR_MSG)?;
        }

        error_if!(
            &blobs_or_uploads != &"blobs",
            "Element 2 or 4 of resource_name should have been 'blobs'. Got: {}",
            blobs_or_uploads
        );
        let hash = &parts.next().err_tip(|| ERROR_MSG)?;
        let raw_digest_size = parts.next().err_tip(|| ERROR_MSG)?;
        let expected_size = raw_digest_size.parse::<usize>().err_tip(|| {
            format!(
                "Digest size_bytes was not convertable to usize. Got: {}",
                raw_digest_size
            )
        })?;
        Ok(ResourceInfo {
            _instance_name: instance_name,
            _uuid: uuid,
            hash,
            expected_size,
        })
    }
}

struct WriteRequestStreamWrapper {
    stream: Streaming<WriteRequest>,
    current_msg: WriteRequest,
    hash: String,
    expected_size: usize,
    is_first: bool,
    bytes_received: usize,
}

impl WriteRequestStreamWrapper {
    async fn from(mut stream: Streaming<WriteRequest>) -> Result<WriteRequestStreamWrapper, Error> {
        let current_msg = stream
            .message()
            .await
            .err_tip(|| "Error receiving first message in stream")?
            .err_tip(|| "Expected WriteRequest struct in stream")?;

        let resource_info = ResourceInfo::new(&current_msg.resource_name)
            .err_tip(|| "Could not extract resource info from first message of stream")?;
        let hash = resource_info.hash.to_string();
        let expected_size = resource_info.expected_size;
        Ok(WriteRequestStreamWrapper {
            stream,
            current_msg,
            hash,
            expected_size,
            is_first: true,
            bytes_received: 0,
        })
    }

    async fn next<'a>(&'a mut self) -> Result<Option<&'a WriteRequest>, Error> {
        if self.is_first {
            self.is_first = false;
            self.bytes_received += self.current_msg.data.len();
            return Ok(Some(&self.current_msg));
        }
        if self.current_msg.finish_write {
            error_if!(
                self.bytes_received != self.expected_size,
                "Did not send enough data. Expected {}, but so far received {}",
                self.expected_size,
                self.bytes_received
            );
            return Ok(None); // Previous message said it was the last msg.
        }
        error_if!(
            self.bytes_received > self.expected_size,
            "Sent too much data. Expected {}, but so far received {}",
            self.expected_size,
            self.bytes_received
        );
        self.current_msg = self
            .stream
            .message()
            .await
            .err_tip(|| format!("Stream error at byte {}", self.bytes_received))?
            .err_tip(|| "Expected WriteRequest struct in stream")?;
        self.bytes_received += self.current_msg.data.len();

        Ok(Some(&self.current_msg))
    }
}

#[tonic::async_trait]
impl ByteStream for ByteStreamServer {
    type ReadStream = ReadStream;
    async fn read(&self, grpc_request: Request<ReadRequest>) -> Result<Response<Self::ReadStream>, Status> {
        log::info!("\x1b[0;31mRead Req\x1b[0m: {:?}", grpc_request.get_ref());
        let now = Instant::now();
        let resp = self
            .inner_read(grpc_request)
            .await
            .err_tip(|| format!("Failed on read() command"))
            .map_err(|e| e.into());
        let d = now.elapsed().as_secs_f32();
        log::info!("\x1b[0;31mRead Resp\x1b[0m: {}", d);
        resp
    }

    async fn write(&self, grpc_request: Request<Streaming<WriteRequest>>) -> Result<Response<WriteResponse>, Status> {
        log::info!("\x1b[0;31mWrite Req\x1b[0m: {:?}", grpc_request.get_ref());
        let now = Instant::now();
        let resp = self
            .inner_write(grpc_request)
            .await
            .err_tip(|| format!("Failed on write() command"))
            .map_err(|e| e.into());
        let d = now.elapsed().as_secs_f32();
        log::info!("\x1b[0;31mWrite Resp\x1b[0m: {} {:?}", d, resp);
        resp
    }

    async fn query_write_status(
        &self,
        _grpc_request: Request<QueryWriteStatusRequest>,
    ) -> Result<Response<QueryWriteStatusResponse>, Status> {
        log::info!("query_write_status {:?}", _grpc_request.get_ref());
        Err(Status::unimplemented(""))
    }
}