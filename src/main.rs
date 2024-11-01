use actix_web::{web, App, HttpServer, HttpResponse, Error};
use actix_files as fs;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::path::{Path, PathBuf};
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::fs::File;
use std::io::BufReader;

use std::io::{self, ErrorKind};
use log::{debug, error, info};

const DOWNLOADS_DIR: &str = "./downloads";
const METADATA_FILE: &str = "./downloads/metadata.json";

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Download {
    id: String,
    url: String,
    status: String,
    progress: i32,
    file_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DownloadRequest {
    url: String,
}

struct AppState {
    downloads: Arc<Mutex<HashMap<String, Download>>>,
}

async fn start_download(
    data: web::Data<AppState>,
    download_req: web::Json<DownloadRequest>,
) -> Result<HttpResponse, Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let download = Download {
        id: id.clone(),
        url: download_req.url.clone(),
        status: "pending".to_string(),
        progress: 0,
        file_name: None,
    };

    // Сохраняем информацию о загрузке
    {
        let mut downloads = data.downloads.lock().await;
        downloads.insert(id.clone(), download.clone());
        save_metadata(&downloads).await;
    }

    // Запускаем загрузку в отдельном потоке
    let downloads = data.downloads.clone();
    let url = download_req.url.clone();
    
    tokio::spawn(async move {
        let output_dir = PathBuf::from(DOWNLOADS_DIR);
        std::fs::create_dir_all(&output_dir).unwrap();

        let mut cmd = Command::new("you-get")
            .arg("--output-dir")
            .arg(&output_dir)
            .arg(&url)
            .spawn()
            .expect("Failed to start download");

        let status = cmd.wait().expect("Failed to wait for download");

        let mut downloads = downloads.lock().await;
        let download = downloads.get_mut(&id).unwrap();

        if status.success() {
            // Находим скачанный файл в директории
            if let Ok(entries) = std::fs::read_dir(&output_dir) {
                // Берем самый новый файл в директории
                if let Some(entry) = entries
                    .filter_map(|e| e.ok())
                    .max_by_key(|e| e.metadata().unwrap().modified().unwrap())
                {
                    download.file_name = Some(entry.file_name().to_string_lossy().into_owned());
                }
            }
            
            download.status = "completed".to_string();
            download.progress = 100;
        } else {
            download.status = "failed".to_string();
        }

        save_metadata(&downloads).await;
    });

    Ok(HttpResponse::Ok().json(download))
}

async fn get_downloads(data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let downloads = data.downloads.lock().await;
    let downloads_vec: Vec<Download> = downloads.values().cloned().collect();
    Ok(HttpResponse::Ok().json(downloads_vec))
}

async fn delete_download(
    data: web::Data<AppState>,
    id: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let mut downloads = data.downloads.lock().await;
    
    if let Some(download) = downloads.remove(id.as_str()) {
        // Удаляем файл, если он существует
        if let Some(file_name) = download.file_name {
            let file_path = Path::new(DOWNLOADS_DIR).join(file_name);
            if let Err(e) = std::fs::remove_file(file_path) {
                log::error!("Failed to delete file: {}", e);
            }
        }
        save_metadata(&downloads).await;
        Ok(HttpResponse::Ok().finish())
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}


// Функция для проверки и создания необходимых директорий
async fn ensure_directories() -> io::Result<()> {
    debug!("Checking directory: {}", DOWNLOADS_DIR);
    
    // Проверяем существование директории
    if !Path::new(DOWNLOADS_DIR).exists() {
        debug!("Creating directory: {}", DOWNLOADS_DIR);
        std::fs::create_dir_all(DOWNLOADS_DIR).map_err(|e| {
            error!("Failed to create downloads directory: {}", e);
            e
        })?;
    }

    // Проверяем права на запись
    let metadata = std::fs::metadata(DOWNLOADS_DIR).map_err(|e| {
        error!("Failed to get metadata for downloads directory: {}", e);
        e
    })?;

    debug!("Directory permissions: {:?}", metadata.permissions());

    let test_file_path = format!("{}/test_write", DOWNLOADS_DIR);
    match File::create(&test_file_path) {
        Ok(_) => {
            std::fs::remove_file(&test_file_path)?;
            debug!("Write test successful");
        }
        Err(e) => {
            error!("Failed write test to downloads directory: {}", e);
            return Err(io::Error::new(ErrorKind::PermissionDenied, "Cannot write to downloads directory"));
        }
    }

    Ok(())
}

fn load_metadata() -> HashMap<String, Download> {
    debug!("Loading metadata from: {}", METADATA_FILE);
    
    // Проверяем существование файла метаданных
    if !Path::new(METADATA_FILE).exists() {
        debug!("Metadata file doesn't exist, creating new one");
        match std::fs::write(METADATA_FILE, "{}") {
            Ok(_) => debug!("Created empty metadata file"),
            Err(e) => error!("Failed to create metadata file: {}", e)
        }
        return HashMap::new();
    }

    match File::open(METADATA_FILE) {
        Ok(file) => {
            let reader = BufReader::new(file);
            match serde_json::from_reader(reader) {
                Ok(data) => {
                    debug!("Successfully loaded metadata");
                    data
                }
                Err(e) => {
                    error!("Failed to parse metadata file: {}", e);
                    HashMap::new()
                }
            }
        }
        Err(e) => {
            error!("Failed to open metadata file: {}", e);
            HashMap::new()
        }
    }
}

async fn save_metadata(downloads: &HashMap<String, Download>) {
    debug!("Saving metadata to: {}", METADATA_FILE);
    
    match File::create(METADATA_FILE) {
        Ok(file) => {
            match serde_json::to_writer_pretty(file, downloads) {
                Ok(_) => debug!("Successfully saved metadata"),
                Err(e) => error!("Failed to write metadata: {}", e)
            }
        }
        Err(e) => error!("Failed to create metadata file: {}", e)
    }
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // env_logger::init(); 

    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();

    // Проверяем директории и права доступа
    match ensure_directories().await {
        Ok(_) => info!("Directories checked and ready"),
        Err(e) => {
            error!("Failed to ensure directories: {}", e);
            return Err(e);
        }
    }


    // Загружаем существующие метаданные
    let downloads = Arc::new(Mutex::new(load_metadata()));

    let app_state = web::Data::new(AppState {
        downloads: downloads.clone(),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(fs::Files::new("/", "static").index_file("index.html"))
            .service(
                web::scope("/api")
                    .route("/downloads", web::post().to(start_download))
                    .route("/downloads", web::get().to(get_downloads))
                    .route("/downloads/{id}", web::delete().to(delete_download))
            )
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}