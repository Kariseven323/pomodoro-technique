/** UI 运行态：仅前端维护的窗口模式等状态（不持久化）。 */

import { writable } from "svelte/store";

/** 是否处于迷你模式（对应后端 `set_mini_mode`）。 */
export const miniMode = writable<boolean>(false);
