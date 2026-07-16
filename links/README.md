# Links

Application module that offers the ability to create public links to users files and let them still be left encrypted.

The way it works:

User selects a file for sharing, and creates a link:
 - New random link key is created
 - File name, thumbnail and whatever else metadata is all encrypted with the link key and saved in the link
 - The file key is also encrypted with the link key and is a part of the link
 - The link key itself is wrapped under the owner's key so only they can recover it later
 - User shares the link URL with the link key in the URL fragment (`#link-key`)
 - The fragment never leaves the browser, so the server never sees the link key
 - The recipient's browser uses it to decrypt the metadata and file key, then decrypts the downloaded chunks on the fly

The server only ever serves ciphertext for public links; all decryption happens client-side.
