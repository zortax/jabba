#[derive(Debug, Error)]
pub enum Error<T: std::error::Error> {
  #[error("The given type does not have a binding")]
  NoBinding,

  #[error("Failed to instantiate bound implementation: {0}")]
  InstanceCreationFailed(#[from] T),
}

#[derive(Error, Debug)]
#[error("unreachable")]
pub struct Infallible;
