# Links

Application module that offers the ability to create public links to users files and let them still be left encrypted.

The way it works:

User selects a file for sharing, and creates a link:
 - New random AES key for the link is created
 - File name, thumbnail and whatever else metadata is all encrypted with `link_aes_key` and saved in the link
 - `file_aes_key` is encrypted also with that `link_aes_key` and is a part of the link
 - User shares the link + plaintext `link_aes_key` to whoever he wants to
 - When the link is accessed, the `link_aes_key` is used to decrypt the data about the file on the server
 - Stream downloading starts and the unencrypted `file_aes_key` is used to decrypt the file on the fly


Although this will technically expose the `file_aes_key` to the server (in memory) it is chosen as a more secure 
approach then simply sharing the link with original `file_aes_key`. 