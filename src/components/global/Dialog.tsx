import { JSX, Show } from "solid-js";
import { Portal } from "solid-js/web";

import styles from "./Dialog.module.css";

export const dialogStyles = styles;

type BtnType = "danger" | "ok" | "attention" | "info";
interface PromptDialogButton {
  type?: BtnType;
  text?: string;
  callback?: () => void;
}
interface PromptDialogOptions {
  title?: string | null;
  question: string;
  btns?: {
    ok?: PromptDialogButton;
    cancel?: PromptDialogButton;
  };
}

function getBtnTypeClass(type?: BtnType) {
  switch (type) {
    case "attention":
      return styles.dialog__btnsBtnAttention;
    case "danger":
      return styles.dialog__btnsBtnDanger;
    case "ok":
      return styles.dialog__btnsBtnOk;
    case "info":
      return styles.dialog__btnsBtnInfo;
    default:
      return "";
  }
}

export type DismissCallback = () => void;

export function PromptDialog({ options }: { options: PromptDialogOptions }) {
  return (
    <Dialog>
      <div class={styles.dialog__container}>
        <Show when={options.title !== undefined}>
          <h2 class={styles.dialog__title}>{options.title ?? "Confirm Decision"}</h2>
        </Show>
        <p class={styles.dialog__message}>{options.question}</p>
        <div class={styles.dialog__btns}>
          <button
            on:click={options?.btns?.ok?.callback}
            classList={{ [styles.dialog__btnsBtn]: true, [getBtnTypeClass(options?.btns?.ok?.type)]: true }}
          >
            {options?.btns?.ok?.text ?? "Confirm"}
          </button>
          <button
            on:click={options?.btns?.cancel?.callback}
            classList={{ [styles.dialog__btnsBtn]: true, [getBtnTypeClass(options?.btns?.cancel?.type)]: true }}
          >
            {options.btns?.cancel?.text ?? "Cancel"}
          </button>
        </div>
      </div>
    </Dialog>
  );
}

export function DefaultDialog(props: {
  onDismiss?: DismissCallback;
  class?: JSX.HTMLAttributes<HTMLDivElement>["class"];
  children?: JSX.Element;
}) {
  return (
    <Dialog onDismiss={props.onDismiss}>
      <div class={`${styles.dialog__container} ${props.class ?? ""}`}>{props.children}</div>
    </Dialog>
  );
}

export function InfoDialog({ title, message }: { title: string | null; message: string }) {
  return (
    <PromptDialog
      options={{
        title,
        question: message,
        btns: {
          cancel: { text: "Ok" },
        },
      }}
    />
  );
}

export default function Dialog(props: { onDismiss?: DismissCallback; children: JSX.Element }) {
  function onClick(e: MouseEvent) {
    if (e.eventPhase !== Event.BUBBLING_PHASE) {
      props.onDismiss!();
    }
  }

  return (
    <Portal>
      <div class={styles.dialog} on:click={props.onDismiss && onClick}>
        {props.children}
      </div>
    </Portal>
  );
}
