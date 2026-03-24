/**
 * wasm-pack's generated package.json only lists transfer.js in sideEffects.
 * Bundlers then tree-shake transfer_bg.js and drop __wbindgen_closure_* exports
 * that are only imported by the .wasm file → WebAssembly.instantiate() fails with
 * "function import requires a callable".
 */
import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const pkgPath = join(dirname(fileURLToPath(import.meta.url)), '..', 'pkg', 'package.json')
const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'))
pkg.sideEffects = ['./transfer.js', './transfer_bg.js', './transfer_bg.wasm', './snippets/*']
writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n')
