import { readFile } from 'fs/promises';

const wasmBytes = await readFile('./rad_wasm.wasm');
const module = await WebAssembly.compile(wasmBytes);
const imports = WebAssembly.Module.imports(module);
console.log('WASM Imports:', JSON.stringify(imports, null, 2));
const exports = WebAssembly.Module.exports(module);
console.log('WASM Exports:', JSON.stringify(exports, null, 2));
