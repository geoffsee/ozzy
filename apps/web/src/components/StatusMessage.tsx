import type { MessageState } from "./types";

type StatusMessageProps = {
  message: MessageState;
};

export function StatusMessage({ message }: StatusMessageProps) {
  return (
    <p id="msg" className={message.isError ? "error" : "muted"}>
      {message.text}
    </p>
  );
}
