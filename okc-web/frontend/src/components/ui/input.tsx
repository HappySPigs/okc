import { forwardRef, type InputHTMLAttributes, type LabelHTMLAttributes, type TextareaHTMLAttributes } from "react";
import { cn } from "@/lib/cn";

const FIELD =
  "w-full rounded-lg border border-line bg-surface px-3 py-2 text-sm placeholder:text-faint " +
  "focus:border-accent-line disabled:opacity-50";

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  function Input({ className, ...props }, ref) {
    return <input ref={ref} className={cn(FIELD, className)} {...props} />;
  },
);

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaHTMLAttributes<HTMLTextAreaElement>>(
  function Textarea({ className, ...props }, ref) {
    return <textarea ref={ref} className={cn(FIELD, "min-h-20 resize-y", className)} {...props} />;
  },
);

export function Label({ className, ...props }: LabelHTMLAttributes<HTMLLabelElement>) {
  return <label className={cn("text-[13px] font-medium text-fg", className)} {...props} />;
}
