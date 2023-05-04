# Storage

Application storage service that manages files and folders in the database and is used to receive uploaded files and store them with the storage 
provider. 

The files are chunked by the frontend and uploaded incrementally. Each chunk cannot be larger then the `storage::CHUNK_SIZE_BYTES` constant.