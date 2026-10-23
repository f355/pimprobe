// PIMProbe - touch probing for the Nestworks C500.
// Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

import {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
  type ButtonHTMLAttributes,
  type ReactNode,
} from "react";
import {
  ArrowLeft,
  ArrowRight,
  BookOpen,
  CircleHelp,
  CornerDownLeft,
  Delete,
  X,
} from "lucide-react";
import { useTranslation } from "./i18n";
import { helpProps } from "./help/context";
import { actionHelp } from "./help/text";

export const HeaderHelpContext = createContext({
  active: false,
  toggle: () => {},
  guide: () => {},
});
export interface PageHeaderProps {
  title: string;
  detail?: string;
  onBack: () => void;
  backDisabled?: boolean;
}
export const HeaderRendererContext = createContext<
  ((props: PageHeaderProps) => ReactNode) | null
>(null);
export function HelpButton() {
  const { t } = useTranslation();
  const help = useContext(HeaderHelpContext);
  const label = t(help.active ? "Exit help" : "Help");
  return (
    <Button
      className={`pp-icon pp-help-button ${help.active ? "pp-selected" : ""}`}
      aria-label={label}
      helpTitle={label}
      data-help-bypass=""
      aria-pressed={help.active}
      onClick={help.toggle}
    >
      {help.active ? <X /> : <CircleHelp />}
    </Button>
  );
}
export function Button({
  help,
  helpTitle,
  primary,
  danger,
  className = "",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  help?: string;
  helpTitle?: string;
  primary?: boolean;
  danger?: boolean;
}) {
  const { t } = useTranslation();
  const title =
    helpTitle ||
    (typeof props.children === "string"
      ? props.children
      : props["aria-label"] || "");
  return (
    <button
      {...props}
      onPointerDown={(event) => {
        props.onPointerDown?.(event);
        event.preventDefault();
      }}
      className={`pp-button ${primary ? "pp-primary" : ""} ${danger ? "pp-danger" : ""} ${className}`}
      {...helpProps(title, help || t(actionHelp(title, t)))}
    />
  );
}
export function PageHeader({
  title,
  detail,
  onBack,
  backDisabled = false,
}: PageHeaderProps) {
  const { t } = useTranslation();
  const help = useContext(HeaderHelpContext);
  const renderHeader = useContext(HeaderRendererContext);
  if (renderHeader)
    return (
      <>
        {renderHeader({ title, detail, onBack, backDisabled })}
        <div className="pp-section-heading">
          <h1>{title}</h1>
          <span className="pp-muted">{detail}</span>
          {help.active && (
            <Button data-help-bypass="" onClick={help.guide}>
              <BookOpen />
              {t("Guide")}
            </Button>
          )}
        </div>
      </>
    );
  return (
    <header className="pp-page-header">
      <Button
        className="pp-icon pp-back"
        aria-label={t("Back")}
        helpTitle={t("Back")}
        disabled={backDisabled}
        onClick={onBack}
      >
        <ArrowLeft />
      </Button>
      <h1>{title}</h1>
      <span className="pp-muted">{detail}</span>
      {help.active && (
        <Button
          className="pp-guide-button"
          helpTitle={t("Guide")}
          data-help-bypass=""
          onClick={help.guide}
        >
          <BookOpen />
          {t("Guide")}
        </Button>
      )}
      <HelpButton />
    </header>
  );
}
export function Notice({
  children,
  error = false,
}: {
  children: ReactNode;
  error?: boolean;
}) {
  return (
    <div
      role={error ? "alert" : "status"}
      className={`pp-notice ${error ? "pp-error" : ""}`}
    >
      {children}
    </div>
  );
}
export function Switch({
  label,
  checked,
  onChange,
  disabled,
  help,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  help?: string;
}) {
  return (
    <button
      type="button"
      className="pp-switch-row"
      role="switch"
      aria-label={label}
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      {...helpProps(label, help || "")}
    >
      <span>{label}</span>
      <span className="pp-switch">
        <span />
      </span>
    </button>
  );
}
export function ConfirmDialog({
  title,
  message,
  onConfirm,
  onCancel,
  confirmLabel,
  danger,
  busy,
  actions,
  children,
}: {
  title: string;
  message: ReactNode;
  onConfirm: () => void;
  onCancel: () => void;
  confirmLabel?: string;
  danger?: boolean;
  busy?: boolean;
  actions?: ReactNode;
  children?: ReactNode;
}) {
  const { t } = useTranslation();
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const dialog = ref.current!;
    dialog.showModal();
    return () => dialog.close();
  }, []);
  return (
    <dialog
      ref={ref}
      className="pp-dialog"
      aria-label={title}
      onCancel={(e) => {
        e.preventDefault();
        if (!busy) onCancel();
      }}
    >
      <h2>{title}</h2>
      <div className="pp-dialog-message">{message}</div>
      {children}
      <div className="pp-dialog-actions">
        <Button disabled={busy} onClick={onCancel}>
          {t("Cancel")}
        </Button>
        {actions}
        <Button
          primary={!danger}
          danger={danger}
          disabled={busy}
          onClick={onConfirm}
        >
          {confirmLabel || t("Proceed")}
        </Button>
      </div>
    </dialog>
  );
}

type Edit = {
  input: HTMLInputElement;
  value: number;
  min: number;
  max: number;
  integer: boolean;
  onChange: (value: number) => void | Promise<void>;
};
const NumericContext = createContext<{
  edit: Edit | null;
  error: string;
  pending: boolean;
  begin: (edit: Edit) => void;
  cancel: () => void;
  accept: () => Promise<boolean>;
}>(null!);
export function NumericProvider({ children }: { children: ReactNode }) {
  const [edit, setEdit] = useState<Edit | null>(null);
  const editRef = useRef<Edit | null>(null);
  const [error, setError] = useState("");
  const [pending, setPending] = useState(false);
  const saving = useRef(false);
  function cancel() {
    if (saving.current) return;
    const input = editRef.current?.input;
    if (input && editRef.current) {
      input.value = String(editRef.current.value);
      input.setSelectionRange(0, 0);
      input.blur();
    }
    editRef.current = null;
    setEdit(null);
    setError("");
  }
  function begin(next: Edit) {
    if (editRef.current?.input === next.input) return;
    cancel();
    editRef.current = next;
    setEdit(next);
    setError("");
    next.input.focus({ preventScroll: true });
    next.input.select();
  }
  async function accept() {
    const current = editRef.current;
    if (!current) return true;
    if (saving.current) return false;
    const value = Number(current.input.value);
    if (
      !current.input.value.trim() ||
      !Number.isFinite(value) ||
      value < current.min ||
      value > current.max ||
      (current.integer && !Number.isInteger(value))
    ) {
      setError(
        `${current.min} – ${current.max}${current.integer ? " · whole numbers" : ""}`,
      );
      return false;
    }
    saving.current = true;
    setPending(true);
    setError("");
    try {
      await current.onChange(value);
      editRef.current = null;
      setEdit(null);
      current.input.setSelectionRange(0, 0);
      current.input.blur();
      return true;
    } catch (failure) {
      setError((failure as Error).message);
      return false;
    } finally {
      saving.current = false;
      setPending(false);
    }
  }
  return (
    <NumericContext.Provider
      value={{ edit, error, pending, begin, cancel, accept }}
    >
      {children}
    </NumericContext.Provider>
  );
}
export const useNumeric = () => useContext(NumericContext);
export function NumberField({
  label,
  value,
  onChange,
  min = 0,
  max = 100000,
  integer = false,
  disabled = false,
  help,
}: {
  label: string;
  value: number;
  onChange: (value: number) => void | Promise<void>;
  min?: number;
  max?: number;
  integer?: boolean;
  disabled?: boolean;
  help?: string;
  unit?: string;
}) {
  const ref = useRef<HTMLInputElement>(null);
  const numeric = useNumeric();
  useEffect(() => {
    if (ref.current && numeric.edit?.input !== ref.current)
      ref.current.value = String(value);
  }, [value, numeric.edit]);
  return (
    <input
      ref={ref}
      className="pp-number"
      type="text"
      inputMode="none"
      aria-label={label}
      defaultValue={value}
      autoComplete="off"
      spellCheck={false}
      disabled={disabled || numeric.pending}
      {...helpProps(label, help || "")}
      onPointerDown={(e) => {
        if (numeric.edit?.input !== ref.current) {
          e.preventDefault();
          numeric.begin({
            input: ref.current!,
            value,
            onChange,
            min,
            max,
            integer,
          });
        }
      }}
      onFocus={() => {
        if (numeric.edit?.input !== ref.current)
          numeric.begin({
            input: ref.current!,
            value,
            onChange,
            min,
            max,
            integer,
          });
      }}
      onBlur={() => {
        if (!numeric.pending) numeric.cancel();
      }}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          void numeric.accept();
        }
        if (e.key === "Escape") numeric.cancel();
      }}
    />
  );
}
export function KeypadSlot({ children }: { children: ReactNode }) {
  const numeric = useNumeric();
  const { t } = useTranslation();
  function key(key: string) {
    const input = numeric.edit?.input;
    if (!input) return;
    input.focus({ preventScroll: true });
    const start = input.selectionStart ?? input.value.length,
      end = input.selectionEnd ?? start;
    let cursor = start;
    if (key === "left" || key === "right")
      cursor =
        key === "left"
          ? Math.max(0, start === end ? start - 1 : start)
          : Math.min(input.value.length, start === end ? end + 1 : end);
    else if (key === "sign") {
      const negative = input.value.startsWith("-");
      input.value = negative ? input.value.slice(1) : "-" + input.value;
      cursor = Math.max(0, start + (negative ? -1 : 1));
    } else if (key === "backspace") {
      const from = start === end ? Math.max(0, start - 1) : start;
      input.value = input.value.slice(0, from) + input.value.slice(end);
      cursor = from;
    } else {
      input.value = input.value.slice(0, start) + key + input.value.slice(end);
      cursor = start + key.length;
    }
    input.setSelectionRange(cursor, cursor);
  }
  return (
    <div className="pp-keypad-slot">
      {numeric.edit ? (
        <div className="pp-keypad-wrap">
          <div
            className="pp-keypad"
            role="group"
            aria-label={t("Numeric keypad")}
          >
            {[
              "1",
              "2",
              "3",
              "left",
              "4",
              "5",
              "6",
              "right",
              "7",
              "8",
              "9",
              "backspace",
              "sign",
              "0",
              ".",
              "enter",
            ].map((value) => (
              <Button
                key={value}
                className={value === "enter" ? "pp-enter" : ""}
                disabled={numeric.pending}
                aria-label={t(
                  (
                    {
                      left: "Move cursor left",
                      right: "Move cursor right",
                      backspace: "Backspace",
                      sign: "Change sign",
                      enter: "Enter",
                    } as Record<string, string>
                  )[value] || value,
                )}
                onPointerDown={(e) => e.preventDefault()}
                onClick={() =>
                  value === "enter" ? void numeric.accept() : key(value)
                }
              >
                {value === "left" ? (
                  <ArrowLeft />
                ) : value === "right" ? (
                  <ArrowRight />
                ) : value === "backspace" ? (
                  <Delete />
                ) : value === "enter" ? (
                  <CornerDownLeft />
                ) : value === "sign" ? (
                  "±"
                ) : (
                  value
                )}
              </Button>
            ))}
          </div>
          {numeric.error && <Notice error>{numeric.error}</Notice>}
        </div>
      ) : (
        children
      )}
    </div>
  );
}
