use std::{env, process, fs, path::{Path, PathBuf}, fs::File, io::Read};
use walkdir::WalkDir;
use zip::write::FileOptions;
use std::io::prelude::*;

struct Directories
{
    current_prefix : u32,
    list : Vec<CustomDirectory>
}
struct CustomDirectory
{
    prefix : u32,
    size : f32,
    files : Vec<String>
}
struct CustomError
{
    msg : String
}
struct Params 
{
    from_dir: PathBuf,
    to_dir :PathBuf,
    log :bool,
    max_part_size_mb : f32
}
fn verify_params(args : Vec<String>) -> Result<Params, CustomError>
{
    if args.len() <5
    {
        return Err(CustomError{msg: String::from("Número de parámetros incorrecto")});
    }
    let from_dir : &str = &args[1];
    let from_dir : &Path = Path::new(&from_dir);
    if !from_dir.exists()
    {
        return Err(CustomError{msg: String::from("Directorio origen no existe")});
    }
    let to_dir : &str = &args[2];
    let to_dir : &Path =Path::new(&to_dir);
    if !to_dir.exists()
    {
        return Err(CustomError{msg: String::from("Directorio destino no existe")});
    }
    let mut to_dir_content : fs::ReadDir = match to_dir.read_dir() {
        Ok(dir_content) => dir_content,
        Err(_) => return Err(CustomError{msg: String::from("Error al leer el directorio destino")})
    };
    if !to_dir_content.next().is_none()
    {
        return Err(CustomError{msg: String::from("Directorio destino no está vacio")});
    }
    let max_part_size_mb : f32 =match &args[3].parse::<f32>() {
        Ok(mb) => mb.to_owned(),
        Err(_) => return Err(CustomError{msg: String::from("Parámetro de tamaño inválido")})
    };
    let log : &str = &args[4];
    if log !="true" && log !="false"
    {
        return Err(CustomError{msg: String::from("Parámetro de registro inválido")})
    }
    let log : bool = {
        if log =="true"
        {
            true
        }
        else
        {
            false
        }
    };
    Ok(Params{
        from_dir:  from_dir.to_owned(),
        to_dir: to_dir.to_owned(),
        max_part_size_mb :max_part_size_mb.to_owned(),
        log: log
    })
}
fn split_dir(params : &Params, mut directories : Directories) -> Result<Directories, CustomError>
{
    for path in WalkDir::new(&params.from_dir).into_iter().filter_map(|e| e.ok())
    {
        let size_mb : f32 = match fs::metadata(path.path().display().to_string()) 
        {
            Ok(meta) => {
                let size_bytes : u64 = meta.len();
                let size_bytes : f32 = size_bytes as f32;
                let size_mb : f32 = (size_bytes/1024.0)/1024.0;
                size_mb
            },
            Err(e) => continue
        };
        if size_mb > params.max_part_size_mb
        {
            let mut msg : String =String::from("El archivo ");
            msg.push_str(&path.path().display().to_string());
            msg.push_str(" supera el peso máximo del paquete");
            return Err(CustomError{msg: msg});
        }
        match set_file_to_available_dir(params, directories, path, &size_mb)
        {
            Ok(dirs) => directories =dirs,
            Err(e) => {
                let msg : String =String::from("Error al procesar el archivo");
                return Err(CustomError{msg: msg});
            }
        };
    }
    return Ok(directories);
}
fn set_file_to_available_dir(params : &Params, mut directories : Directories, path : walkdir::DirEntry, size_file : &f32) -> Result<Directories, CustomError>
{
    let mut current_directory : Option<&CustomDirectory> = Option::None;
    let mut dir_counter : usize=0;
    for dir in directories.list.iter_mut()
    {            
        if dir.size < params.max_part_size_mb && 
            &(params.max_part_size_mb - dir.size) > size_file
        {
            dir.size = dir.size + size_file;
            dir.files.push(
                path.path().display().to_string()
            );
            current_directory = Some(dir);
        }
        dir_counter = dir_counter+1;
    }
    if current_directory.is_none()
    {
        directories.list.push(
            CustomDirectory{
                prefix: directories.current_prefix.to_owned(),
                size : 0.0,
                files : Vec::new()
            }
        );
        let aux_current_directory : &mut CustomDirectory = match directories.list.last_mut() {
            Some(last_dir) => last_dir,
            None => return Err(CustomError{msg: String::from("No se pudo crear el nuevo directorio lógico")})
        };
        aux_current_directory.size = aux_current_directory.size + size_file;
        aux_current_directory.files.push(
                path.path().display().to_string()
            );
        current_directory =Some(aux_current_directory);
        let mut new_dir_path : String= String::from(params.to_dir.display().to_string());
        new_dir_path.push_str("/");
        match fs::create_dir_all(new_dir_path) {
            Ok(res) =>{},
            Err(e) => return Err(CustomError{msg: String::from("No se pudo crear el nuevo directorio físico")})
        };
        directories.current_prefix = directories.current_prefix +1;
    }
    match current_directory {
        Some(dir) => {
            
            let file_path :String =path.path().display().to_string();
            let from_path :String =params.from_dir.display().to_string();
            
            let to_path   :String =params.to_dir.display().to_string();
            let to_path =Path::new(&to_path);
            let to_path = to_path.join(Path::new(&dir.prefix.to_string()));
            let to_path : String =to_path.display().to_string();

            let dest_file_path : String =str::replace(
                &file_path,
                &from_path,
                &to_path
            );
            let filename :String = Path::new(&dest_file_path).file_name().unwrap().to_os_string().to_str().unwrap().to_string();

            let dest_path_without_filename = str::replace(&dest_file_path, &filename, "");

            match fs::create_dir_all(Path::new(&dest_path_without_filename)) {
                Ok(_) =>{},
                Err(_) => return Err(CustomError{msg: String::from("No se pudo crear el nuevo directorio físico")})
            };

            let dest_file_path = Path::new(&dest_file_path);

            match fs::copy(path.path(), dest_file_path) {
                Ok(_) => {},
                Err(_) => {}
            };
        },
        None => return Err(CustomError{msg: String::from("No se pudo establecer el directorio actual")})
    };
    if !params.log
    {
        directories = clear_dirs(directories);
    }
    return Ok(directories);
}
fn clear_dirs(mut directories : Directories) -> Directories
{
    if directories.list.len() >= 2
    {
        directories.list.remove(0);
    }
    return directories;
}
fn zip_dirs(params : &Params, total_dirs : &u32) -> Result<bool, std::io::Error>
{
    let dirs =fs::read_dir(params.to_dir.to_owned()).unwrap();
    let mut dirs_processed : u32 =0;
    for dir in dirs
    {
        let current_path =dir?.path();
        let mut dst_file_path :String =String::from(&current_path.display().to_string());
        dst_file_path.push_str(".zip");
        let dst_file_path : &Path = Path::new(&dst_file_path);
        let zip_file = File::create(dst_file_path).unwrap();
        let mut zip_file =zip::ZipWriter::new(zip_file);
        let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Bzip2)
        .unix_permissions(0o755);
        let mut buffer : Vec<u8> = Vec::new();
        for sub_dir in WalkDir::new(&current_path)
        {
            let sub_dir_path =sub_dir?.path().to_owned();
            let sub_dir_path_str : String =sub_dir_path.display().to_string();
            let mut sub_dir_path_str =sub_dir_path_str.replace(&params.to_dir.display().to_string(), "");
            
            let path_components = Path::new(&sub_dir_path_str).iter();
            let mut new_path_components : Vec<&str> = Vec::new();
            let mut index_components =0;
            for path_component in path_components
            {
                if index_components >1
                {
                    match path_component.to_str()
                    {
                        Some(e) =>{
                            new_path_components.push(e);
                        },
                        (_) =>{}
                    }
                }
                index_components = index_components+1;
            }
            let mut new_sub_dir_path = String::from("/");
            for new_path_component in new_path_components
            {
                let aux_new_sub_dir_path =Path::new(&new_sub_dir_path);
                let aux_new_path_component =Path::new(new_path_component);
                
                let aux_new_path_component_joined =aux_new_sub_dir_path.join(aux_new_path_component);

                new_sub_dir_path =aux_new_path_component_joined.display().to_string();
            }
            sub_dir_path_str =new_sub_dir_path;
            if sub_dir_path.is_file()
            {
                zip_file.start_file(sub_dir_path_str, options);
                let mut file = File::open(sub_dir_path)?;
                file.read_to_end(&mut buffer);
                zip_file.write_all(&*buffer);
                buffer.clear();
            }
            else
            {
                zip_file.add_directory(sub_dir_path_str, options)?;
            }
        }
        zip_file.finish()?;
        fs::remove_dir_all(current_path);
        dirs_processed =dirs_processed+1;
        println!("{} de {} Directorios procesados", dirs_processed, total_dirs);

    }
    return Ok(true);
}
fn main() {
    let args : Vec<String> = env::args().collect();
    let params : Params = match verify_params(args) {
        Ok(parms) => parms,
        Err(e) => {
            println!("{:?}", e.msg);
            process::exit(1);
        }
    };
    println!("Separando directorios...",);
    let mut directories : Directories = Directories{
        current_prefix: 0,
        list : Vec::new()};
    let directories : Directories = match split_dir(&params, directories) {
        Ok(dirs) => dirs,
        Err(e) =>{
            println!("{:?}", e.msg);
            process::exit(1);
        }
    };
    println!("Comprimiendo {} directorios...", directories.current_prefix);
    let zipped_dirs : bool = match zip_dirs(&params, &directories.current_prefix)
    {
        Ok(zipped_dirs) => zipped_dirs,
        Err(e) =>
        {
            println!("{:?}", e);
            process::exit(1);
        }
    };
    println!("Operación finalizada con éxito");
}
