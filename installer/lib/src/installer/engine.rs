// 安装引擎

use crate::common::{Error, Result, process::ProcessDetector};
use crate::installer::{InstallState, InstallTask};
use crate::resources::manifest::InstallManifest;

/// 安装引擎
pub struct InstallEngine {
    state: InstallState,
    tasks: Vec<Box<dyn InstallTask>>,
}

impl InstallEngine {
    /// 创建新的安装引擎
    pub fn new(install_path: String) -> Self {
        Self {
            state: InstallState::new(install_path),
            tasks: Vec::new(),
        }
    }
    
    /// 获取状态
    pub fn state(&self) -> &InstallState {
        &self.state
    }
    
    /// 添加任务
    pub fn add_task(&mut self, task: Box<dyn InstallTask>) {
        self.tasks.push(task);
    }

    /// 检查目标进程是否正在运行
    pub fn check_target_processes(&self, target_processes: &[String]) -> Result<bool> {
        let detector = ProcessDetector::new(target_processes.to_vec());
        detector.is_target_running()
    }

    /// 终止目标进程
    pub fn terminate_target_processes(&self, target_processes: &[String]) -> Result<()> {
        let detector = ProcessDetector::new(target_processes.to_vec());
        detector.terminate_target_processes()
    }

    /// 请求目标进程优雅退出
    pub fn request_target_processes_exit(&self, target_processes: &[String]) -> Result<()> {
        let detector = ProcessDetector::new(target_processes.to_vec());
        detector.request_target_processes_exit()
    }
    
    /// 执行安装
    pub async fn install(&self) -> Result<InstallManifest> {
        use crate::logger::{log_step, StepStatus};
        
        tracing::info!("Starting installation to: {}", self.state.install_path());
        
        let mut completed_tasks = Vec::new();
        
        // 执行所有任务
        for (i, task) in self.tasks.iter().enumerate() {
            if self.state.is_cancelled() {
                tracing::warn!("Installation cancelled by user");
                self.rollback_tasks(&completed_tasks).await?;
                return Err(Error::UserCancelled);
            }
            
            let task_name = task.name();
            log_step(task_name, StepStatus::Started);
            
            match task.execute(&self.state) {
                Ok(_) => {
                    log_step(task_name, StepStatus::Completed);
                    completed_tasks.push(i);
                }
                Err(e) => {
                    log_step(task_name, StepStatus::Failed(e.to_string()));
                    tracing::error!("Task '{}' failed: {}", task_name, e);
                    
                    // 回滚已完成的任务
                    self.rollback_tasks(&completed_tasks).await?;
                    
                    return Err(Error::InstallationFailed(format!(
                        "Task '{}' failed: {}",
                        task_name, e
                    )));
                }
            }
        }
        
        // 创建安装清单
        let manifest = InstallManifest::new(
            "MyApp".to_string(), // 这应该从配置读取
            "1.0.0".to_string(),
            self.state.install_path().into(),
            crate::i18n::current_locale(),
        );
        
        // 保存清单
        let manifest_path = std::path::Path::new(&self.state.install_path())
            .join("install_manifest.json");
        manifest.save(&manifest_path)?;
        
        tracing::info!("Installation completed successfully");
        
        Ok(manifest)
    }
    
    /// 回滚任务
    async fn rollback_tasks(&self, completed_task_indices: &[usize]) -> Result<()> {
        use crate::logger::{log_step, StepStatus};
        
        tracing::warn!("Rolling back {} completed tasks", completed_task_indices.len());
        
        for &index in completed_task_indices.iter().rev() {
            if let Some(task) = self.tasks.get(index) {
                let task_name = task.name();
                tracing::info!("Rolling back task: {}", task_name);
                
                if let Err(e) = task.rollback(&self.state) {
                    tracing::error!("Rollback failed for task '{}': {}", task_name, e);
                    // 继续回滚其他任务
                }
            }
        }
        
        Ok(())
    }
}

