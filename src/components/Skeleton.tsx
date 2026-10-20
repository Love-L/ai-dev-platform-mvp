import { motion } from "framer-motion";

export function CardSkeleton() {
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      className="rounded-xl p-6"
      style={{
        background: "var(--color-surface)",
        border: "1px solid var(--color-border)",
        boxShadow: "var(--shadow-sm)",
      }}
    >
      <div className="animate-pulse">
        <div className="flex items-center gap-4 mb-4">
          <div className="w-12 h-12 rounded-lg" style={{ background: "var(--color-bg-subtle)" }} />
          <div className="flex-1">
            <div className="h-4 rounded w-1/3 mb-2" style={{ background: "var(--color-bg-subtle)" }} />
            <div className="h-6 rounded w-1/2" style={{ background: "var(--color-bg-subtle)" }} />
          </div>
        </div>
        <div className="space-y-3">
          <div className="h-4 rounded" style={{ background: "var(--color-bg-subtle)" }} />
          <div className="h-4 rounded w-5/6" style={{ background: "var(--color-bg-subtle)" }} />
          <div className="h-4 rounded w-4/6" style={{ background: "var(--color-bg-subtle)" }} />
        </div>
      </div>
    </motion.div>
  );
}

export function StatCardSkeleton() {
  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      className="rounded-xl p-6"
      style={{
        background: "var(--color-surface)",
        border: "1px solid var(--color-border)",
        boxShadow: "var(--shadow-sm)",
      }}
    >
      <div className="animate-pulse">
        <div className="flex items-center gap-4">
          <div className="w-12 h-12 rounded-lg" style={{ background: "var(--color-bg-subtle)" }} />
          <div className="flex-1">
            <div className="h-4 rounded w-1/2 mb-2" style={{ background: "var(--color-bg-subtle)" }} />
            <div className="h-8 rounded w-1/3" style={{ background: "var(--color-bg-subtle)" }} />
          </div>
        </div>
      </div>
    </motion.div>
  );
}
