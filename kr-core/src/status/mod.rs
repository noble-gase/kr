use serde::Serialize;

#[derive(Serialize)]
pub struct Status<T>
where
    T: Serialize + Send,
{
    pub code: i32,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

#[macro_export]
macro_rules! define_ok {
    ($code:expr, $msg:expr) => {
        pub struct OK<T>(pub Option<T>)
        where
            T: Serialize + Send;

        impl<T> OK<T>
        where
            T: Serialize + Send,
        {
            pub fn to_status(self) -> Status<T> {
                Status {
                    code: $code,
                    msg: String::from($msg),
                    data: self.0,
                }
            }
        }
    };
}

#[macro_export]
macro_rules! define_error_codes {
    ($($name:ident($code:expr, $msg:expr)),* $(,)?) => {
        #[derive(Debug, thiserror::Error)]
        pub enum Code {
            #[error("[{0}] {1}")]
            Custom(i32, String),
            $(
                #[error("[$code] $msg")]
                $name,
            )*
        }

        impl Code {
            pub fn code(&self) -> i32 {
                match self {
                    Code::Custom(c, _) => *c,
                    $(
                        Code::$name => $code,
                    )*
                }
            }

            pub fn with_msg(&self, msg: impl Into<String>) -> Self {
                Code::Custom(self.code(), msg.into())
            }

            pub fn to_status(self) -> Status<()> {
                Status {
                    code: self.code(),
                    msg: self.to_string(),
                    data: None,
                }
            }
        }
    };
}
