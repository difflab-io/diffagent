import type { EmptyStateVariants } from './styles'

/** Props for the app-agnostic `EmptyState` placeholder primitive. */
export interface EmptyStateProps extends EmptyStateVariants {
  /** Primary heading displayed inside the empty pane. */
  title: string
  /** Optional supporting guidance displayed beneath the heading. */
  description?: string
  /** Optional class override merged onto the root element. */
  class?: string
}
