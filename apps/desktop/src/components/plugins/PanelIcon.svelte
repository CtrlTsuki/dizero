<script lang="ts">
  import { ChartBar, Notebook, List, Sparkles, Clock, Star, Puzzle, Bell, Bookmark, Zap } from "@lucide/svelte";

  let { name, size = 15 }: { name: string | null; size?: number } = $props();

  // Panel icons are an allowlist validated by the manifest parser.
  const icons = {
    "bar-chart": ChartBar,
    notebook: Notebook,
    list: List,
    sparkles: Sparkles,
    clock: Clock,
    star: Star,
    puzzle: Puzzle,
    bell: Bell,
    bookmark: Bookmark,
    zap: Zap,
  } as const;

  const Icon = $derived(icons[(name ?? "puzzle") as keyof typeof icons] ?? Puzzle);
</script>

<Icon {size} strokeWidth={1.8} />
