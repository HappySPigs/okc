import * as S from "@radix-ui/react-switch";
import { cn } from "@/lib/cn";

export function Switch({
  checked, onCheckedChange, disabled, id, className, "data-testid": testid,
}: {
  checked?: boolean;
  onCheckedChange?: (v: boolean) => void;
  disabled?: boolean;
  id?: string;
  className?: string;
  "data-testid"?: string;
}) {
  return (
    <S.Root
      id={id}
      checked={checked}
      onCheckedChange={onCheckedChange}
      disabled={disabled}
      data-testid={testid}
      className={cn(
        "relative inline-flex h-5 w-9 shrink-0 items-center rounded-full border border-line transition-colors",
        "data-[state=checked]:bg-accent data-[state=unchecked]:bg-subtle disabled:opacity-50",
        className,
      )}
    >
      <S.Thumb className="block size-4 translate-x-0.5 rounded-full bg-surface shadow transition-transform data-[state=checked]:translate-x-[18px]" />
    </S.Root>
  );
}
