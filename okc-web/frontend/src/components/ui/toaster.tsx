import { Toaster as SonnerToaster, toast } from "sonner";

export { toast };

export function Toaster() {
  return (
    <SonnerToaster
      position="bottom-right"
      toastOptions={{
        classNames: {
          toast: "!rounded-lg !border !border-line !bg-surface !text-fg !text-sm",
        },
      }}
    />
  );
}
