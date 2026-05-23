import { readFile } from 'fs/promises';

export interface RadStorageBackend {
  put(key: string, data: string): void;
  get(key: string): string | null;
  list(prefix: string): string[];
  delete(key: string): void;
}

export class RadWasm {
  private instance: WebAssembly.Instance;
  private memory: WebAssembly.Memory;
  private storageBackend: RadStorageBackend;
  private textEncoder = new TextEncoder();
  private textDecoder = new TextDecoder();

  private constructor(
    instance: WebAssembly.Instance,
    memory: WebAssembly.Memory,
    storageBackend: RadStorageBackend
  ) {
    this.instance = instance;
    this.memory = memory;
    this.storageBackend = storageBackend;
  }

  static async load(
    wasmPath: string,
    storageBackend: RadStorageBackend
  ): Promise<RadWasm> {
    const wasmBytes = await readFile(wasmPath);
    // Create a mutable reference to hold the memory
    const memoryRef: { current: WebAssembly.Memory | null } = { current: null };
    const getMemory = () => {
      if (!memoryRef.current) throw new Error('Memory not initialized');
      return memoryRef.current;
    };

    const importObject = {
      // wasm-bindgen stubs (required by getrandom crate)
      __wbindgen_placeholder__: {
        __wbindgen_describe: () => {},
        __wbg___wbindgen_throw_9c31b086c2b26051: (ptr: number, len: number) => {
          const msg = readString(getMemory(), ptr, len);
          throw new Error(`WASM error: ${msg}`);
        },
      },
      __wbindgen_externref_xform__: {
        __wbindgen_externref_table_set_null: () => {},
        __wbindgen_externref_table_grow: () => 0,
      },
      env: {
        storage_put: (
          keyPtr: number,
          keyLen: number,
          dataPtr: number,
          dataLen: number
        ): number => {
          try {
            const key = readString(getMemory(), keyPtr, keyLen);
            const data = readString(getMemory(), dataPtr, dataLen);
            storageBackend.put(key, data);
            return 0;
          } catch (e) {
            console.error('storage_put error:', e);
            return -1;
          }
        },
        storage_get: (
          keyPtr: number,
          keyLen: number,
          resultPtrPtr: number,
          resultLenPtr: number
        ): number => {
          try {
            const memory = getMemory();
            const key = readString(memory, keyPtr, keyLen);
            const data = storageBackend.get(key);
            if (data === null) {
              return -1; // not found
            }
            // Write result to WASM memory
            const bytes = new TextEncoder().encode(data);
            const ptr = (
              importObject.env as any
            )._allocResult(bytes.length);
            new Uint8Array(memory.buffer, ptr, bytes.length).set(bytes);

            // Write ptr and len to output parameters
            const view = new DataView(memory.buffer);
            view.setUint32(resultPtrPtr, ptr, true);
            view.setUint32(resultLenPtr, bytes.length, true);
            return 0;
          } catch (e) {
            console.error('storage_get error:', e);
            return -2;
          }
        },
        storage_list: (
          prefixPtr: number,
          prefixLen: number,
          resultPtrPtr: number,
          resultLenPtr: number
        ): number => {
          try {
            const memory = getMemory();
            const prefix = readString(memory, prefixPtr, prefixLen);
            const keys = storageBackend.list(prefix);
            const json = JSON.stringify(keys);
            const bytes = new TextEncoder().encode(json);
            const ptr = (
              importObject.env as any
            )._allocResult(bytes.length);
            new Uint8Array(memory.buffer, ptr, bytes.length).set(bytes);

            const view = new DataView(memory.buffer);
            view.setUint32(resultPtrPtr, ptr, true);
            view.setUint32(resultLenPtr, bytes.length, true);
            return 0;
          } catch (e) {
            console.error('storage_list error:', e);
            return -1;
          }
        },
        storage_delete: (keyPtr: number, keyLen: number): number => {
          try {
            const key = readString(getMemory(), keyPtr, keyLen);
            storageBackend.delete(key);
            return 0;
          } catch (e) {
            console.error('storage_delete error:', e);
            return -1;
          }
        },
        _allocResult: (size: number): number => {
          const exports = importObject.env as any;
          return exports._radAlloc ? exports._radAlloc(size) : 0;
        },
      },
    };

    const { instance } = await WebAssembly.instantiate(
      wasmBytes,
      importObject
    );

    // Store exports for later use
    (importObject.env as any)._radAlloc = instance.exports.rad_alloc;

    // Use the memory exported by WASM
    const memory = instance.exports.memory as WebAssembly.Memory;
    memoryRef.current = memory;

    return new RadWasm(instance, memory, storageBackend);
  }

  private writeString(s: string): [number, number] {
    const bytes = this.textEncoder.encode(s);
    const ptr = (this.instance.exports.rad_alloc as Function)(bytes.length);
    new Uint8Array(this.memory.buffer, ptr, bytes.length).set(bytes);
    return [ptr, bytes.length];
  }

  private readResult(): string {
    const ptr = (this.instance.exports.rad_result_ptr as Function)();
    const len = (this.instance.exports.rad_result_len as Function)();
    const bytes = new Uint8Array(this.memory.buffer, ptr, len);
    return this.textDecoder.decode(bytes);
  }

  init(): string {
    (this.instance.exports.rad_init as Function)();
    return this.readResult();
  }

  join(input: string): string {
    const [ptr, len] = this.writeString(input);
    (this.instance.exports.rad_join as Function)(ptr, len);
    return this.readResult();
  }

  submitOp(input: string): string {
    const [ptr, len] = this.writeString(input);
    (this.instance.exports.rad_submit_op as Function)(ptr, len);
    return this.readResult();
  }

  accept(input: string): string {
    const [ptr, len] = this.writeString(input);
    (this.instance.exports.rad_accept as Function)(ptr, len);
    return this.readResult();
  }

  getLog(input: string = '{}'): string {
    const [ptr, len] = this.writeString(input);
    (this.instance.exports.rad_get_log as Function)(ptr, len);
    return this.readResult();
  }

  compact(): string {
    (this.instance.exports.rad_compact as Function)();
    return this.readResult();
  }

  getParticipants(): string {
    (this.instance.exports.rad_get_participants as Function)();
    return this.readResult();
  }

  getFile(path: string): string {
    const [ptr, len] = this.writeString(path);
    (this.instance.exports.rad_get_file as Function)(ptr, len);
    return this.readResult();
  }

  getRegions(path: string): string {
    const [ptr, len] = this.writeString(path);
    (this.instance.exports.rad_get_regions as Function)(ptr, len);
    return this.readResult();
  }

  getOpStatus(id: string): string {
    const [ptr, len] = this.writeString(id);
    (this.instance.exports.rad_get_op_status as Function)(ptr, len);
    return this.readResult();
  }

  getOp(id: string): string {
    const [ptr, len] = this.writeString(id);
    (this.instance.exports.rad_get_op as Function)(ptr, len);
    return this.readResult();
  }

  getVisible(path: string): string {
    const [ptr, len] = this.writeString(path);
    (this.instance.exports.rad_get_visible as Function)(ptr, len);
    return this.readResult();
  }

  getFileList(): string {
    (this.instance.exports.rad_get_file_list as Function)();
    return this.readResult();
  }
}

export function createMemoryBackend(): RadStorageBackend {
  const storage = new Map<string, string>();

  return {
    put(key: string, data: string): void {
      storage.set(key, data);
    },
    get(key: string): string | null {
      return storage.get(key) ?? null;
    },
    list(prefix: string): string[] {
      const keys: string[] = [];
      for (const key of storage.keys()) {
        if (key.startsWith(prefix)) {
          keys.push(key);
        }
      }
      return keys;
    },
    delete(key: string): void {
      storage.delete(key);
    },
  };
}

function readString(
  memory: WebAssembly.Memory,
  ptr: number,
  len: number
): string {
  const bytes = new Uint8Array(memory.buffer, ptr, len);
  return new TextDecoder().decode(bytes);
}
