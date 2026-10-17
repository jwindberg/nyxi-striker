use crate::db::DbManager;
use crate::device_manager::DeviceManager;
use crate::proto::driver::driver_grpc_server::DriverGrpc;
use crate::proto::driver::*;
use crate::system::SystemMonitor;
use futures_core::Stream;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status};

pub struct DriverGrpcService {
    db: DbManager,
    device_mgr: Arc<DeviceManager>,
    system_monitor: Arc<SystemMonitor>,
    mic_muted: AtomicBool,
    wireless_loop_locked: AtomicBool,
}

impl DriverGrpcService {
    pub fn new(db: DbManager, device_mgr: Arc<DeviceManager>, system_monitor: Arc<SystemMonitor>) -> Self {
        Self {
            db,
            device_mgr,
            system_monitor,
            mic_muted: AtomicBool::new(false),
            wireless_loop_locked: AtomicBool::new(false),
        }
    }
}

type ResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send + 'static>>;

#[tonic::async_trait]
impl DriverGrpc for DriverGrpcService {
    type watchDevListStream = ResponseStream<DeviceList>;

    async fn watch_dev_list(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::watchDevListStream>, Status> {
        let initial = self.device_mgr.current_device_list();
        let rx = self.device_mgr.subscribe();
        let broadcast_stream = BroadcastStream::new(rx).filter_map(|res| res.ok());

        // Emit initial immediately followed by broadcast changes
        let s = async_stream::stream! {
            yield Ok(initial);
            for await item in broadcast_stream {
                yield Ok(item);
            }
        };

        Ok(Response::new(Box::pin(s) as Self::watchDevListStream))
    }

    type watchVenderStream = ResponseStream<VenderMsg>;

    async fn watch_vender(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::watchVenderStream>, Status> {
        let s = async_stream::stream! {
            // Placeholder vendor event stream
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                yield Ok(VenderMsg { msg: Vec::new() });
            }
        };

        Ok(Response::new(Box::pin(s) as Self::watchVenderStream))
    }

    type watchSystemInfoStream = ResponseStream<SystemInfo>;

    async fn watch_system_info(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::watchSystemInfoStream>, Status> {
        let monitor = self.system_monitor.clone();
        let s = async_stream::stream! {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(1000));
            loop {
                interval.tick().await;
                let info = monitor.snapshot();
                yield Ok(info);
            }
        };

        Ok(Response::new(Box::pin(s) as Self::watchSystemInfoStream))
    }

    async fn send_msg(&self, request: Request<SendMsg>) -> Result<Response<ResSend>, Status> {
        let req = request.into_inner();
        let checksum_type = CheckSumType::try_from(req.checksumtype).unwrap_or(CheckSumType::None);
        let dangle_dev_type = DangleDevType::try_from(req.dangledevtype).unwrap_or(DangleDevType::Dangledevtypenone);

        tracing::info!(
            "send_msg: dev={}, len={}, opcode=0x{:02x?}, cs={:?}, dangle={:?}",
            req.devicepath,
            req.msg.len(),
            req.msg.first(),
            checksum_type,
            dangle_dev_type
        );

        let err = match self.device_mgr.send_msg(&req.devicepath, req.msg, checksum_type, dangle_dev_type) {
            Ok(_) => {
                tracing::info!("send_msg SUCCESS");
                "".to_string()
            }
            Err(e) => {
                tracing::warn!("send_msg error: {}", e);
                e
            }
        };

        Ok(Response::new(ResSend { err }))
    }

    async fn read_msg(&self, request: Request<ReadMsg>) -> Result<Response<ResRead>, Status> {
        let req = request.into_inner();
        tracing::info!("read_msg: dev={}", req.devicepath);
        match self.device_mgr.read_msg(&req.devicepath) {
            Ok(msg) => {
                tracing::info!("read_msg SUCCESS: len={}, first={:02x?}", msg.len(), msg.first());
                Ok(Response::new(ResRead {
                    err: "".to_string(),
                    msg,
                }))
            }
            Err(e) => {
                tracing::warn!("read_msg error: {}", e);
                Ok(Response::new(ResRead {
                    err: e,
                    msg: Vec::new(),
                }))
            }
        }
    }

    async fn send_raw_feature(&self, request: Request<SendMsg>) -> Result<Response<ResSend>, Status> {
        self.send_msg(request).await
    }

    async fn read_raw_feature(&self, request: Request<ReadMsg>) -> Result<Response<ResRead>, Status> {
        self.read_msg(request).await
    }

    async fn set_light_type(&self, request: Request<SetLight>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        tracing::info!(
            "set_light_type: path={}, type={}, screen_id={}, dangle_dev={}",
            req.devicepath,
            req.lighttype,
            req.screen_id,
            req.dangledevtype
        );
        Ok(Response::new(Empty {}))
    }

    type upgradeOTAGATTStream = ResponseStream<Progress>;

    async fn upgrade_otagatt(
        &self,
        request: Request<OtaUpgrade>,
    ) -> Result<Response<Self::upgradeOTAGATTStream>, Status> {
        let req = request.into_inner();
        tracing::info!("upgrade_otagatt called for dev: {}, buf len: {}", req.devpath, req.file_buf.len());

        let s = async_stream::stream! {
            yield Ok(Progress { progress: 0.0, err: "".to_string() });
            yield Ok(Progress { progress: 100.0, err: "".to_string() });
        };

        Ok(Response::new(Box::pin(s) as Self::upgradeOTAGATTStream))
    }

    async fn mute_microphone(&self, request: Request<MuteMicrophone>) -> Result<Response<ResSend>, Status> {
        let req = request.into_inner();
        self.mic_muted.store(req.need_mute, Ordering::SeqCst);
        Ok(Response::new(ResSend { err: "".to_string() }))
    }

    async fn toggle_microphone_mute(&self, _request: Request<Empty>) -> Result<Response<ResSend>, Status> {
        let prev = self.mic_muted.fetch_xor(true, Ordering::SeqCst);
        tracing::info!("Toggled microphone mute to: {}", !prev);
        Ok(Response::new(ResSend { err: "".to_string() }))
    }

    async fn get_microphone_mute(&self, _request: Request<Empty>) -> Result<Response<MicrophoneMuteStatus>, Status> {
        let is_mute = self.mic_muted.load(Ordering::SeqCst);
        Ok(Response::new(MicrophoneMuteStatus {
            is_mute,
            err: "".to_string(),
        }))
    }

    async fn change_wireless_loop_status(
        &self,
        request: Request<WirelessLoopStatus>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        self.wireless_loop_locked.store(req.lock, Ordering::SeqCst);
        tracing::info!("Wireless loop lock set to: {}", req.lock);
        Ok(Response::new(Empty {}))
    }

    async fn insert_db(&self, request: Request<InsertDb>) -> Result<Response<ResSend>, Status> {
        let req = request.into_inner();
        let err = match self.db.insert(&req.dbpath, &req.key, &req.value) {
            Ok(_) => "".to_string(),
            Err(e) => {
                tracing::error!("insert_db error: {}", e);
                e
            }
        };
        Ok(Response::new(ResSend { err }))
    }

    async fn delete_item_from_db(&self, request: Request<DeleteItem>) -> Result<Response<ResSend>, Status> {
        let req = request.into_inner();
        let err = match self.db.delete(&req.dbpath, &req.key) {
            Ok(_) => "".to_string(),
            Err(e) => {
                tracing::error!("delete_item_from_db error: {}", e);
                e
            }
        };
        Ok(Response::new(ResSend { err }))
    }

    async fn get_item_from_db(&self, request: Request<GetItem>) -> Result<Response<Item>, Status> {
        let req = request.into_inner();
        match self.db.get(&req.dbpath, &req.key) {
            Ok(Some(val)) => Ok(Response::new(Item {
                value: val,
                err_str: "".to_string(),
            })),
            Ok(None) => Ok(Response::new(Item {
                value: Vec::new(),
                err_str: "Key not found".to_string(),
            })),
            Err(e) => Ok(Response::new(Item {
                value: Vec::new(),
                err_str: e,
            })),
        }
    }

    async fn get_all_keys_from_db(&self, request: Request<GetAll>) -> Result<Response<AllList>, Status> {
        let req = request.into_inner();
        match self.db.get_all_keys(&req.dbpath) {
            Ok(keys) => Ok(Response::new(AllList {
                data_list: keys,
                err_str: "".to_string(),
            })),
            Err(e) => Ok(Response::new(AllList {
                data_list: Vec::new(),
                err_str: e,
            })),
        }
    }

    async fn get_all_values_from_db(&self, request: Request<GetAll>) -> Result<Response<AllList>, Status> {
        let req = request.into_inner();
        match self.db.get_all_values(&req.dbpath) {
            Ok(values) => Ok(Response::new(AllList {
                data_list: values,
                err_str: "".to_string(),
            })),
            Err(e) => Ok(Response::new(AllList {
                data_list: Vec::new(),
                err_str: e,
            })),
        }
    }

    async fn clean_dev(&self, request: Request<ReadMsg>) -> Result<Response<ResSend>, Status> {
        let req = request.into_inner();
        self.device_mgr.clean_dev(&req.devicepath);
        Ok(Response::new(ResSend { err: "".to_string() }))
    }
}
