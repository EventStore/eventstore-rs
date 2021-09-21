#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateReq {
    #[prost(message, optional, tag = "1")]
    pub options: ::core::option::Option<create_req::Options>,
}
/// Nested message and enum types in `CreateReq`.
pub mod create_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[prost(string, tag = "4")]
        pub query: ::prost::alloc::string::String,
        #[prost(oneof = "options::Mode", tags = "1, 2, 3")]
        pub mode: ::core::option::Option<options::Mode>,
    }
    /// Nested message and enum types in `Options`.
    pub mod options {
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Transient {
            #[prost(string, tag = "1")]
            pub name: ::prost::alloc::string::String,
        }
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Continuous {
            #[prost(string, tag = "1")]
            pub name: ::prost::alloc::string::String,
            #[prost(bool, tag = "2")]
            pub track_emitted_streams: bool,
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Mode {
            #[prost(message, tag = "1")]
            OneTime(super::super::super::Empty),
            #[prost(message, tag = "2")]
            Transient(Transient),
            #[prost(message, tag = "3")]
            Continuous(Continuous),
        }
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateResp {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateReq {
    #[prost(message, optional, tag = "1")]
    pub options: ::core::option::Option<update_req::Options>,
}
/// Nested message and enum types in `UpdateReq`.
pub mod update_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[prost(string, tag = "1")]
        pub name: ::prost::alloc::string::String,
        #[prost(string, tag = "2")]
        pub query: ::prost::alloc::string::String,
        #[prost(oneof = "options::EmitOption", tags = "3, 4")]
        pub emit_option: ::core::option::Option<options::EmitOption>,
    }
    /// Nested message and enum types in `Options`.
    pub mod options {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum EmitOption {
            #[prost(bool, tag = "3")]
            EmitEnabled(bool),
            #[prost(message, tag = "4")]
            NoEmitOptions(super::super::super::Empty),
        }
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateResp {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteReq {
    #[prost(message, optional, tag = "1")]
    pub options: ::core::option::Option<delete_req::Options>,
}
/// Nested message and enum types in `DeleteReq`.
pub mod delete_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[prost(string, tag = "1")]
        pub name: ::prost::alloc::string::String,
        #[prost(bool, tag = "2")]
        pub delete_emitted_streams: bool,
        #[prost(bool, tag = "3")]
        pub delete_state_stream: bool,
        #[prost(bool, tag = "4")]
        pub delete_checkpoint_stream: bool,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteResp {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StatisticsReq {
    #[prost(message, optional, tag = "1")]
    pub options: ::core::option::Option<statistics_req::Options>,
}
/// Nested message and enum types in `StatisticsReq`.
pub mod statistics_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[prost(oneof = "options::Mode", tags = "1, 2, 3, 4, 5")]
        pub mode: ::core::option::Option<options::Mode>,
    }
    /// Nested message and enum types in `Options`.
    pub mod options {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Mode {
            #[prost(string, tag = "1")]
            Name(::prost::alloc::string::String),
            #[prost(message, tag = "2")]
            All(super::super::super::Empty),
            #[prost(message, tag = "3")]
            Transient(super::super::super::Empty),
            #[prost(message, tag = "4")]
            Continuous(super::super::super::Empty),
            #[prost(message, tag = "5")]
            OneTime(super::super::super::Empty),
        }
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StatisticsResp {
    #[prost(message, optional, tag = "1")]
    pub details: ::core::option::Option<statistics_resp::Details>,
}
/// Nested message and enum types in `StatisticsResp`.
pub mod statistics_resp {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Details {
        #[prost(int64, tag = "1")]
        pub core_processing_time: i64,
        #[prost(int64, tag = "2")]
        pub version: i64,
        #[prost(int64, tag = "3")]
        pub epoch: i64,
        #[prost(string, tag = "4")]
        pub effective_name: ::prost::alloc::string::String,
        #[prost(int32, tag = "5")]
        pub writes_in_progress: i32,
        #[prost(int32, tag = "6")]
        pub reads_in_progress: i32,
        #[prost(int32, tag = "7")]
        pub partitions_cached: i32,
        #[prost(string, tag = "8")]
        pub status: ::prost::alloc::string::String,
        #[prost(string, tag = "9")]
        pub state_reason: ::prost::alloc::string::String,
        #[prost(string, tag = "10")]
        pub name: ::prost::alloc::string::String,
        #[prost(string, tag = "11")]
        pub mode: ::prost::alloc::string::String,
        #[prost(string, tag = "12")]
        pub position: ::prost::alloc::string::String,
        #[prost(float, tag = "13")]
        pub progress: f32,
        #[prost(string, tag = "14")]
        pub last_checkpoint: ::prost::alloc::string::String,
        #[prost(int64, tag = "15")]
        pub events_processed_after_restart: i64,
        #[prost(string, tag = "16")]
        pub checkpoint_status: ::prost::alloc::string::String,
        #[prost(int64, tag = "17")]
        pub buffered_events: i64,
        #[prost(int32, tag = "18")]
        pub write_pending_events_before_checkpoint: i32,
        #[prost(int32, tag = "19")]
        pub write_pending_events_after_checkpoint: i32,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StateReq {
    #[prost(message, optional, tag = "1")]
    pub options: ::core::option::Option<state_req::Options>,
}
/// Nested message and enum types in `StateReq`.
pub mod state_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[prost(string, tag = "1")]
        pub name: ::prost::alloc::string::String,
        #[prost(string, tag = "2")]
        pub partition: ::prost::alloc::string::String,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StateResp {
    #[prost(message, optional, tag = "1")]
    pub state: ::core::option::Option<::prost_types::Value>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ResultReq {
    #[prost(message, optional, tag = "1")]
    pub options: ::core::option::Option<result_req::Options>,
}
/// Nested message and enum types in `ResultReq`.
pub mod result_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[prost(string, tag = "1")]
        pub name: ::prost::alloc::string::String,
        #[prost(string, tag = "2")]
        pub partition: ::prost::alloc::string::String,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ResultResp {
    #[prost(message, optional, tag = "1")]
    pub result: ::core::option::Option<::prost_types::Value>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ResetReq {
    #[prost(message, optional, tag = "1")]
    pub options: ::core::option::Option<reset_req::Options>,
}
/// Nested message and enum types in `ResetReq`.
pub mod reset_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[prost(string, tag = "1")]
        pub name: ::prost::alloc::string::String,
        #[prost(bool, tag = "2")]
        pub write_checkpoint: bool,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ResetResp {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EnableReq {
    #[prost(message, optional, tag = "1")]
    pub options: ::core::option::Option<enable_req::Options>,
}
/// Nested message and enum types in `EnableReq`.
pub mod enable_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[prost(string, tag = "1")]
        pub name: ::prost::alloc::string::String,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EnableResp {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DisableReq {
    #[prost(message, optional, tag = "1")]
    pub options: ::core::option::Option<disable_req::Options>,
}
/// Nested message and enum types in `DisableReq`.
pub mod disable_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[prost(string, tag = "1")]
        pub name: ::prost::alloc::string::String,
        #[prost(bool, tag = "2")]
        pub write_checkpoint: bool,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DisableResp {}
#[doc = r" Generated client implementations."]
pub mod projections_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[derive(Debug, Clone)]
    pub struct ProjectionsClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl ProjectionsClient<tonic::transport::Channel> {
        #[doc = r" Attempt to create a new client by connecting to a given endpoint."]
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> ProjectionsClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::ResponseBody: Body + Send + Sync + 'static,
        T::Error: Into<StdError>,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> ProjectionsClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<http::Request<tonic::body::BoxBody>>>::Error:
                Into<StdError> + Send + Sync,
        {
            ProjectionsClient::new(InterceptedService::new(inner, interceptor))
        }
        #[doc = r" Compress requests with `gzip`."]
        #[doc = r""]
        #[doc = r" This requires the server to support it otherwise it might respond with an"]
        #[doc = r" error."]
        pub fn send_gzip(mut self) -> Self {
            self.inner = self.inner.send_gzip();
            self
        }
        #[doc = r" Enable decompressing responses with `gzip`."]
        pub fn accept_gzip(mut self) -> Self {
            self.inner = self.inner.accept_gzip();
            self
        }
        pub async fn create(
            &mut self,
            request: impl tonic::IntoRequest<super::CreateReq>,
        ) -> Result<tonic::Response<super::CreateResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/event_store.client.projections.Projections/Create",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn update(
            &mut self,
            request: impl tonic::IntoRequest<super::UpdateReq>,
        ) -> Result<tonic::Response<super::UpdateResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/event_store.client.projections.Projections/Update",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn delete(
            &mut self,
            request: impl tonic::IntoRequest<super::DeleteReq>,
        ) -> Result<tonic::Response<super::DeleteResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/event_store.client.projections.Projections/Delete",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn statistics(
            &mut self,
            request: impl tonic::IntoRequest<super::StatisticsReq>,
        ) -> Result<tonic::Response<tonic::codec::Streaming<super::StatisticsResp>>, tonic::Status>
        {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/event_store.client.projections.Projections/Statistics",
            );
            self.inner
                .server_streaming(request.into_request(), path, codec)
                .await
        }
        pub async fn disable(
            &mut self,
            request: impl tonic::IntoRequest<super::DisableReq>,
        ) -> Result<tonic::Response<super::DisableResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/event_store.client.projections.Projections/Disable",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn enable(
            &mut self,
            request: impl tonic::IntoRequest<super::EnableReq>,
        ) -> Result<tonic::Response<super::EnableResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/event_store.client.projections.Projections/Enable",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn reset(
            &mut self,
            request: impl tonic::IntoRequest<super::ResetReq>,
        ) -> Result<tonic::Response<super::ResetResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/event_store.client.projections.Projections/Reset",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn state(
            &mut self,
            request: impl tonic::IntoRequest<super::StateReq>,
        ) -> Result<tonic::Response<super::StateResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/event_store.client.projections.Projections/State",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn result(
            &mut self,
            request: impl tonic::IntoRequest<super::ResultReq>,
        ) -> Result<tonic::Response<super::ResultResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/event_store.client.projections.Projections/Result",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn restart_subsystem(
            &mut self,
            request: impl tonic::IntoRequest<super::super::Empty>,
        ) -> Result<tonic::Response<super::super::Empty>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/event_store.client.projections.Projections/RestartSubsystem",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}
