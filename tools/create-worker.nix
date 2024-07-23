{
  bash,
  buildEnv,
  buildImage,
  coreutils,
  clang,
  go,
  diffutils,
  gnutar,
  gzip,
  lib,
  runCommand,
  runtimeShell,
  self,
  rust,
  ...
}: let
  # A temporary directory. Note that this doesn't set any permissions. Those
  # need to be added explicitly in the final image arguments.
  mkTmp = runCommand "mkTmp" {} ''
    mkdir -p $out/tmp
  '';

  # Permissions for the temporary directory.
  mkTmpPerms = {
    path = mkTmp;
    regex = ".*";
    mode = "1777";
    uid = 0; # Owned by root.
    gid = 0; # Owned by root.
  };

  # Enable the shebang `#!/usr/bin/env bash`.
  mkEnvSymlink = runCommand "mkEnvSymlink" {} ''
    mkdir -p $out/usr/bin
    ln -s /bin/env $out/usr/bin/env
  '';

  user = "nativelink";
  group = "nativelink";
  uid = "1000";
  gid = "1000";

  mkUser = runCommand "mkUser" {} ''
    mkdir -p $out/etc/pam.d

    echo "root:x:0:0::/root:${runtimeShell}" > $out/etc/passwd
    echo "${user}:x:${uid}:${gid}:::" >> $out/etc/passwd

    echo "root:!x:::::::" > $out/etc/shadow
    echo "${user}:!x:::::::" >> $out/etc/shadow

    echo "root:x:0:" > $out/etc/group
    echo "${group}:x:${gid}:" >> $out/etc/group

    echo "root:x::" > $out/etc/gshadow
    echo "${group}:x::" >> $out/etc/gshadow

    cat > $out/etc/pam.d/other <<EOF
    account sufficient pam_unix.so
    auth sufficient pam_rootok.so
    password requisite pam_unix.so nullok sha512
    session required pam_unix.so
    EOF

    #touch $out/etc/login.defs
    mkdir -p $out/home/${user}
    echo "export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" >> $out/etc/bashrc
    echo "export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" >> $out/etc/profile
    # echo "export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" >> $out/.bashrc
    # echo "export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" >> $out/.profile
    # echo "export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" >> $out/home/${user}/.bashrc
    # echo "export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" >> $out/home/${user}/.profile
  '';

  # Set permissions for the user's home directory.
  mkUserPerms = {
    path = mkUser;
    regex = "/home/${user}";
    mode = "0755";
    uid = lib.toInt uid;
    gid = lib.toInt gid;
    uname = user;
    gname = group;
  };
in
  # Create a container image from a base image with the nativelink executable
  # added and set as entrypoint. This allows arbitrary base images to be
  # "enriched" with nativelink to create worker images for cloud deployments.
  image:
    buildImage {
      name = "nativelink-worker-${image.imageName}";
      # Note this removes ability to use the passed arguments from the create-worker invocations.
      # fromImage = image;
      maxLayers = 20;
      copyToRoot = [
        mkUser
        mkTmp
        mkEnvSymlink
        (buildEnv {
          name = "${image.imageName}-buildEnv";
          paths = [coreutils bash rust clang go diffutils gnutar gzip];
          pathsToLink = ["/bin"];
        })
      ];

      perms = [
        mkUserPerms
        mkTmpPerms
      ];

      # Override the final image tag with the one from the base image to make
      # the relationship between the toolchain and the worker extension more
      # obvious.
      tag = image.imageTag;

      config = {
        # Entrypoint = [ "/bin/bash -l" ];
        User = user;
        WorkingDir = "/home/${user}";
        # Env = [
        #   "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
        # ];
        Labels = {
          "org.opencontainers.image.description" = "NativeLink worker generated from ${image.imageName}.";
          "org.opencontainers.image.documentation" = "https://github.com/TraceMachina/nativelink";
          "org.opencontainers.image.licenses" = "Apache-2.0";
          "org.opencontainers.image.revision" = "${self.rev or self.dirtyRev or "dirty"}";
          "org.opencontainers.image.source" = "https://github.com/TraceMachina/nativelink";
          "org.opencontainers.image.title" = "NativeLink worker for ${image.imageName}";
          "org.opencontainers.image.vendor" = "Trace Machina, Inc.";
        };
      };
    }
