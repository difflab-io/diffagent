<script lang="ts">
  import { Ark } from '@ark-ui/svelte/factory'
  import { cn } from '@cloudvoyant/helical-ui'
  import { emptyStateVariants } from './styles'
  import type { EmptyStateProps } from './types'

  // Props ---------------------------------------------------------------------
  let { title, description, size = 'md', class: className = '' }: EmptyStateProps = $props()

  // Derived -------------------------------------------------------------------
  const classes = $derived(cn(emptyStateVariants({ size }), className))
</script>

<Ark as="section" class={classes} data-slot="empty-state" aria-label={title}>
  <h1 class="text-base font-medium text-foreground">{title}</h1>
  {#if description}
    <p class="max-w-md text-xs leading-relaxed text-muted-foreground">{description}</p>
  {/if}
</Ark>
