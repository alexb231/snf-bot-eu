use thiserror::Error;



#[derive(Debug, Error)]
#[non_exhaustive]
#[allow(clippy::module_name_repetitions)]
pub enum SFError {
    
    
    
    #[error("Tried to send an invalid request: {0}")]
    InvalidRequest(&'static str),
    
    
    #[error("Received an empty response from the server")]
    EmptyResponse,
    
    
    #[error("Could not communicate with the server")]
    ConnectionError,
    
    
    #[error(
        "Error parsing the server response because {0} had an unexpected \
         value of: {1}"
    )]
    ParsingError(&'static str, String),
    
    
    
    
    
    #[error("Server responded with error: {0}")]
    ServerError(String),
    
    
    #[error("The server version {0} is not supported")]
    UnsupportedVersion(u32),
    
    #[error(
        "Tried to access the response for {name} at [{pos}] , but the \
         response is too short. The response is: {array}"
    )]
    TooShortResponse {
        
        name: &'static str,
        
        pos: usize,
        
        array: String,
    },
    
    #[error("Multiple errors occurred:\n{}", .0.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n"))]
    NestedError(Vec<SFError>),
}
