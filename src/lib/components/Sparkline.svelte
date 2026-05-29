<script lang="ts">
  interface Props {
    values: number[];
    width?: number;
    height?: number;
    color?: string;
  }

  const { values, width = 64, height = 18, color = '#06d6a0' }: Props = $props();

  const points = $derived((() => {
    if (values.length < 2) return '';
    const max = Math.max(...values, 1);
    const xStep = width / (values.length - 1);
    return values
      .map((v, i) => `${(i * xStep).toFixed(1)},${(height - (v / max) * height).toFixed(1)}`)
      .join(' ');
  })());
</script>

{#if points}
  <svg
    {width}
    {height}
    class="sparkline"
    viewBox="0 0 {width} {height}"
    aria-hidden="true"
  >
    <polyline
      {points}
      fill="none"
      stroke={color}
      stroke-width="1.5"
      stroke-linejoin="round"
      stroke-linecap="round"
      opacity="0.8"
    />
  </svg>
{/if}

<style>
  .sparkline {
    display: block;
    flex-shrink: 0;
  }
</style>
