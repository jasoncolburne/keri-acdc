pub type Result<T> = anyhow::Result<T>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error decoding.")]
    Decoding,
    #[error("Error encoding.")]
    Encoding,
    #[error("Out of order.")]
    OutOfOrder,
    #[error("Programmer error.")]
    Programmer,
    #[error("Error validating schema: {0}")]
    SchemaValidation(String),
    #[error("Value error.")]
    Value,
    #[error("Error verifying.")]
    Verification,
    #[error("Error validating.")]
    Validation,
}

macro_rules! err {
    ($e:expr) => {
        Err($e.into())
    };
}

pub(crate) use err;
