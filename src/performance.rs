use std::time::{Duration, Instant};
use sysinfo::System;

pub struct PerformanceMonitor {
    system: System,
    last_update: Instant,
    update_interval: Duration,
    
    // System metrics
    cpu_usage: f32,
    memory_usage: f32,
    memory_total: u64,
    memory_used: u64,
    
    // App metrics
    frame_count: u64,
    frame_time: Duration,
    fps: f32,
    last_frame_time: Instant,
    
    // GPU 
    gpu_memory_used: Option<u64>,
    gpu_memory_total: Option<u64>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
            last_update: Instant::now(),
            update_interval: Duration::from_millis(500), // Update
            
            cpu_usage: 0.0,
            memory_usage: 0.0,
            memory_total: 0,
            memory_used: 0,
            
            frame_count: 0,
            frame_time: Duration::ZERO,
            fps: 0.0,
            last_frame_time: Instant::now(),
            
            gpu_memory_used: None,
            gpu_memory_total: None,
        }
    }
    
    pub fn update(&mut self) {
        let now = Instant::now();
        
        if now.duration_since(self.last_update) >= self.update_interval {
            self.system.refresh_all();
            
            self.cpu_usage = self.system.global_cpu_info().cpu_usage();
            
            self.memory_total = self.system.total_memory();
            self.memory_used = self.system.used_memory();
            self.memory_usage = if self.memory_total > 0 {
                (self.memory_used as f32 / self.memory_total as f32) * 100.0
            } else {
                0.0
            };
            
            self.last_update = now;
        }
        
        let current_frame_time = now.duration_since(self.last_frame_time);
        self.frame_time = current_frame_time;
        self.frame_count += 1;
        
        if self.frame_time.as_secs_f32() > 0.0 {
            let current_fps = 1.0 / self.frame_time.as_secs_f32();
            self.fps = self.fps * 0.9 + current_fps * 0.1;
        }
        
        self.last_frame_time = now;
    }
    
    pub fn get_stats(&self) -> PerformanceStats {
        PerformanceStats {
            cpu_usage: self.cpu_usage,
            memory_usage: self.memory_usage,
            memory_used_mb: self.memory_used / 1024 / 1024,
            memory_total_mb: self.memory_total / 1024 / 1024,
            fps: self.fps,
            frame_time_ms: self.frame_time.as_secs_f32() * 1000.0,
            frame_count: self.frame_count,
        }
    }
    
    pub fn set_gpu_memory(&mut self, used: u64, total: u64) {
        self.gpu_memory_used = Some(used);
        self.gpu_memory_total = Some(total);
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub fps: f32,
    pub frame_time_ms: f32,
    pub frame_count: u64,
} 