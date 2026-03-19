interface StatCardProps {
  label: string;
  value: string | null;
}

export function StatCard({ label, value }: StatCardProps) {
  return (
    <div className="bg-white p-5 rounded-xl border border-black/[0.07] shadow-sm">
      <p className="text-[10px] font-bold text-black/[0.26] tracking-[0.06em] uppercase mb-1.5">
        {label}
      </p>
      {value !== null ? (
        <p className="text-[28px] font-semibold text-black/85">{value}</p>
      ) : (
        <p className="text-[24px] font-medium text-[rgba(0,0,0,0.25)]">&mdash;</p>
      )}
    </div>
  );
}
