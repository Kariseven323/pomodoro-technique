/** `canvas-confetti` 的简化类型声明（项目内使用到 `default export` 与 `reset()`）。 */

declare module "canvas-confetti" {
  /** `canvas-confetti` 的发射参数（仅声明项目用到的字段）。 */
  export type Options = {
    /** 粒子数量。 */
    particleCount?: number;
    /** 扩散角度。 */
    spread?: number;
    /** 初速度。 */
    startVelocity?: number;
    /** 重力系数。 */
    gravity?: number;
    /** 粒子生命周期 tick 数。 */
    ticks?: number;
    /** 发射原点（0-1 的屏幕比例坐标）。 */
    origin?: { x: number; y: number };
  };

  /** `canvas-confetti` 实例：可调用发射，也可调用 `reset()` 清理。 */
  export type ConfettiInstance = {
    /** 发射一次粒子动画。 */
    (options?: Options): void;
    /** 清理当前画布并停止后续渲染。 */
    reset(): void;
  };

  const confetti: ConfettiInstance;
  export default confetti;
}
