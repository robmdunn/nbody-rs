/* tslint:disable */
/* eslint-disable */
export function main_js(): void;
export class NBodySimulation {
  free(): void;
  /**
   * @param {HTMLCanvasElement} canvas
   * @param {SimConfig} config
   */
  constructor(canvas: HTMLCanvasElement, config: SimConfig);
  step(): void;
  render(): void;
  /**
   * @param {number} x
   * @param {number} y
   */
  handle_mouse_down(x: number, y: number): void;
  /**
   * @param {number} dx
   * @param {number} dy
   */
  handle_mouse_move(dx: number, dy: number): void;
  /**
   * @param {number} delta_y
   */
  handle_scroll(delta_y: number): void;
  /**
   * @param {boolean} show_wireframe
   */
  set_wireframe(show_wireframe: boolean): void;
}
export class SimConfig {
  free(): void;
  constructor();
  fixed_scale: boolean;
  g: number;
  mass: number;
  mode_3d: boolean;
  mzero: number;
  n_bodies: number;
  point_size: number;
  show_wireframe: boolean;
  softening: number;
  spin: number;
  timestep: number;
  tree_ratio: number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_simconfig_free: (a: number, b: number) => void;
  readonly __wbg_get_simconfig_n_bodies: (a: number) => number;
  readonly __wbg_set_simconfig_n_bodies: (a: number, b: number) => void;
  readonly __wbg_get_simconfig_mass: (a: number) => number;
  readonly __wbg_set_simconfig_mass: (a: number, b: number) => void;
  readonly __wbg_get_simconfig_g: (a: number) => number;
  readonly __wbg_set_simconfig_g: (a: number, b: number) => void;
  readonly __wbg_get_simconfig_timestep: (a: number) => number;
  readonly __wbg_set_simconfig_timestep: (a: number, b: number) => void;
  readonly __wbg_get_simconfig_softening: (a: number) => number;
  readonly __wbg_set_simconfig_softening: (a: number, b: number) => void;
  readonly __wbg_get_simconfig_spin: (a: number) => number;
  readonly __wbg_set_simconfig_spin: (a: number, b: number) => void;
  readonly __wbg_get_simconfig_mzero: (a: number) => number;
  readonly __wbg_set_simconfig_mzero: (a: number, b: number) => void;
  readonly __wbg_get_simconfig_tree_ratio: (a: number) => number;
  readonly __wbg_set_simconfig_tree_ratio: (a: number, b: number) => void;
  readonly __wbg_get_simconfig_point_size: (a: number) => number;
  readonly __wbg_set_simconfig_point_size: (a: number, b: number) => void;
  readonly __wbg_get_simconfig_fixed_scale: (a: number) => number;
  readonly __wbg_set_simconfig_fixed_scale: (a: number, b: number) => void;
  readonly __wbg_get_simconfig_mode_3d: (a: number) => number;
  readonly __wbg_set_simconfig_mode_3d: (a: number, b: number) => void;
  readonly __wbg_get_simconfig_show_wireframe: (a: number) => number;
  readonly __wbg_set_simconfig_show_wireframe: (a: number, b: number) => void;
  readonly simconfig_new: () => number;
  readonly __wbg_nbodysimulation_free: (a: number, b: number) => void;
  readonly nbodysimulation_new: (a: number, b: number) => Array;
  readonly nbodysimulation_step: (a: number) => void;
  readonly nbodysimulation_render: (a: number) => void;
  readonly nbodysimulation_handle_mouse_down: (a: number, b: number, c: number) => void;
  readonly nbodysimulation_handle_mouse_move: (a: number, b: number, c: number) => void;
  readonly nbodysimulation_handle_scroll: (a: number, b: number) => void;
  readonly nbodysimulation_set_wireframe: (a: number, b: number) => void;
  readonly main_js: () => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
