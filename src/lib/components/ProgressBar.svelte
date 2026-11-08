<script lang="ts">
  // A thin brass line. With an unknown size (`fraction` null) it pulses at
  // full width instead of growing.
  let { fraction, label }: { fraction: number | null; label: string } = $props();
</script>

<div
  role="progressbar"
  aria-label={label}
  aria-valuemin={0}
  aria-valuemax={100}
  aria-valuenow={fraction === null ? undefined : Math.floor(fraction * 100)}
  class="h-0.5 w-full overflow-hidden bg-wall-line/60"
>
  <div
    class={["h-full bg-brass transition-[width] duration-150 ease-out", fraction === null && "animate-pulse"]}
    style:width="{(fraction ?? 1) * 100}%"
  ></div>
</div>
