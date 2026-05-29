<script lang="ts">
  import { formatBytes } from '$lib/utils/colors';
  import type { DiskInfo } from '$lib/stores/metrics.svelte';
  import MetricRow from './MetricRow.svelte';

  interface Props {
    disk: DiskInfo;
  }

  const { disk }: Props = $props();

  /** Show only the drive letter + colon on Windows, e.g. "C:" from "C:\\" */
  const mountLabel = $derived(
    disk.mount.length >= 2 && disk.mount[1] === ':'
      ? disk.mount.slice(0, 2)
      : disk.mount
  );

  const detail = $derived(`${formatBytes(disk.used)} / ${formatBytes(disk.total)}`);
</script>

<!-- DiskRow delegates to MetricRow — same grid layout, no sparkline (no history). -->
<MetricRow label={mountLabel} percent={disk.percent} {detail} />
