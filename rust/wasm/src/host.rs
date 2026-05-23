/// ホスト関数（WASM から import）
/// TS 側が実装を提供する

extern "C" {
    /// ストレージにデータを書き込む
    /// 戻り値: 0 = success, -1 = error
    pub fn storage_put(
        key_ptr: *const u8,
        key_len: usize,
        data_ptr: *const u8,
        data_len: usize,
    ) -> i32;

    /// ストレージからデータを読み取る
    /// 結果は WASM 線形メモリに書き込まれ、ptr/len を返す
    /// 戻り値: 0 = success, -1 = not found, -2 = error
    pub fn storage_get(
        key_ptr: *const u8,
        key_len: usize,
        result_ptr: *mut *const u8,
        result_len: *mut usize,
    ) -> i32;

    /// プレフィクスでオブジェクト一覧を取得（JSON 配列として返る）
    /// 戻り値: 0 = success, -1 = error
    pub fn storage_list(
        prefix_ptr: *const u8,
        prefix_len: usize,
        result_ptr: *mut *const u8,
        result_len: *mut usize,
    ) -> i32;

    /// ストレージからオブジェクトを削除
    /// 戻り値: 0 = success, -1 = error
    pub fn storage_delete(key_ptr: *const u8, key_len: usize) -> i32;
}
