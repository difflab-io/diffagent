import { cva, type VariantProps } from 'class-variance-authority'

export const emptyStateVariants = cva(
  'helical-empty-state flex flex-1 flex-col items-center justify-center text-center text-muted-foreground select-none',
  {
    variants: {
      size: {
        sm: 'gap-2 p-6',
        md: 'gap-3 p-8',
      },
    },
    defaultVariants: {
      size: 'md',
    },
  },
)

export type EmptyStateVariants = VariantProps<typeof emptyStateVariants>
