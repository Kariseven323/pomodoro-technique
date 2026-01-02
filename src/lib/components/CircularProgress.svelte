<script lang="ts">
  const props = $props<{
    /** 圆环直径（px）。 */
    size: number;
    /** 圆环线宽（px）。 */
    strokeWidth: number;
    /** 进度（0-1）。 */
    progress: number;
    /** 进度颜色。 */
    color: string;
    /** 背景环颜色。 */
    trackColor: string;
  }>();

  /** 将任意数值夹紧到 0-1。 */
  function clamp01(v: number): number {
    if (!Number.isFinite(v)) return 0;
    return Math.max(0, Math.min(1, v));
  }

  /** 圆心坐标（px）。 */
  const cx = $derived(props.size / 2);
  /** 半径（px）：扣除线宽避免溢出。 */
  const r = $derived(Math.max(0, props.size / 2 - props.strokeWidth / 2));
  /** 圆周长（用于 stroke-dasharray）。 */
  const c = $derived(2 * Math.PI * r);
  /** 进度对应的 dash offset（顺时针从 12 点方向开始）。 */
  const dashOffset = $derived(c * (1 - clamp01(props.progress)));
</script>

<svg
  width={props.size}
  height={props.size}
  viewBox={`0 0 ${props.size} ${props.size}`}
  role="img"
  aria-label="计时进度"
>
  <circle {cx} cy={cx} {r} fill="none" stroke={props.trackColor} stroke-width={props.strokeWidth}></circle>
  <circle
    {cx}
    cy={cx}
    {r}
    fill="none"
    stroke={props.color}
    stroke-width={props.strokeWidth}
    stroke-linecap="round"
    stroke-dasharray={c}
    stroke-dashoffset={dashOffset}
    transform={`rotate(-90 ${cx} ${cx})`}
  ></circle>
</svg>
