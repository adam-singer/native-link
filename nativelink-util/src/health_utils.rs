type Name = String;
type Description = String;

#[derive(Debug)]
pub enum HealthStatus {
    Ok(Name,  Description),
    Initializing(Name,  Description),
    Warning(Name,  Description),
    Failed(Name,  Description),
}

pub trait HealthStatusIndicator {
    // TODO: Most likely this will need to be async
    fn check_health(&self) -> HealthStatus;
}
