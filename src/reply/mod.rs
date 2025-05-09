use serde::Serialize;

#[derive(Serialize)]
pub struct Reply<T>
where
    T: Serialize + Send,
{
    pub code: i32,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

// ----------------------------------- salvo -----------------------------------

#[cfg(feature = "salvo")]
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
            pub fn to_reply(self) -> Reply<T> {
                Reply {
                    code: $code,
                    msg: String::from($msg),
                    data: self.0,
                }
            }
        }

        #[async_trait]
        impl<T> Writer for OK<T>
        where
            T: Serialize + Send,
        {
            async fn write(mut self, _req: &mut Request, _depot: &mut Depot, resp: &mut Response) {
                resp.status_code(StatusCode::OK);
                resp.render(Json(self.to_reply()));
            }
        }
    };
}

#[cfg(feature = "salvo")]
#[macro_export]
macro_rules! define_error_codes {
    ($($name:ident($code:expr, $msg:expr)),* $(,)?) => {
        pub enum Code<T>
        where
            T: AsRef<str> + Send,
        {
            Custom(i32, T),
            $(
                $name,
            )*
        }

        impl<T> Code<T>
        where
            T: AsRef<str> + Send,
        {
            pub fn wrap(self, msg: T) -> Self {
                match self {
                    Code::Custom(c, _) => Code::Custom(c, msg),
                    $(
                        Code::$name => Code::Custom($code, msg),
                    )*
                }
            }

            pub fn to_reply(self) -> Reply<()> {
                let (code, msg) = match self {
                    Code::Custom(c, v) => (c, v.as_ref().to_string()),
                    $(
                        Code::$name => ($code, String::from($msg)),
                    )*
                };
                Reply {
                    code,
                    msg,
                    data: None,
                }
            }
        }

        #[async_trait]
        impl<T> Writer for Code<T>
        where
            T: AsRef<str> + Send,
        {
            async fn write(mut self, _req: &mut Request, _depot: &mut Depot, resp: &mut Response) {
                resp.status_code(StatusCode::OK);
                resp.render(Json(self.to_reply()));
            }
        }
    };
}

// ----------------------------------- axum -----------------------------------

#[cfg(feature = "axum")]
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
            pub fn to_reply(self) -> Reply<T> {
                Reply {
                    code: $code,
                    msg: String::from($msg),
                    data: self.0,
                }
            }
        }

        impl<T> IntoResponse for OK<T>
        where
            T: Serialize + Send,
        {
            fn into_response(self) -> Response {
                Json(self.to_reply()).into_response()
            }
        }
    };
}

#[cfg(feature = "axum")]
#[macro_export]
macro_rules! define_error_codes {
    ($($name:ident($code:expr, $msg:expr)),* $(,)?) => {
        pub enum Code<T>
        where
            T: AsRef<str> + Send,
        {
            Custom(i32, T),
            $(
                $name,
            )*
        }

        impl<T> Code<T>
        where
            T: AsRef<str> + Send,
        {
            pub fn wrap(self, msg: T) -> Self {
                match self {
                    Code::Custom(c, _) => Code::Custom(c, msg),
                    $(
                        Code::$name => Code::Custom($code, msg),
                    )*
                }
            }

            pub fn to_reply(self) -> Reply<()> {
                let (code, msg) = match self {
                    Code::Custom(c, v) => (c, v.as_ref().to_string()),
                    $(
                        Code::$name => ($code, String::from($msg)),
                    )*
                };
                Reply {
                    code,
                    msg,
                    data: None,
                }
            }
        }

        impl<T> IntoResponse for Code<T>
        where
            T: AsRef<str> + Send,
        {
            fn into_response(self) -> Response {
                Json(self.to_reply()).into_response()
            }
        }
    };
}
