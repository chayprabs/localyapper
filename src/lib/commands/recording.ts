// IPC wrappers -- the test dictation run by the backend controller
import { invoke } from "@tauri-apps/api/core";

/** The Voice page's test dictation: start or stop. The result arrives as
 *  a `test-dictation-result` event; nothing is pasted or saved. */
export async function testDictation(start: boolean): Promise<void> {
  return invoke<void>("test_dictation", { start });
}
