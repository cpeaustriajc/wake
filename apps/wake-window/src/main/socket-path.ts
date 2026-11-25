import { homedir } from "node:os";
import path from "node:path";

const SOCKET_FILE = "wake.sock";
const STATE_FOLDER = ".wake";

const socketPathOf = (override: unknown): string => {
  if (typeof override === "string") {
    return override;
  }
  return path.join(homedir(), STATE_FOLDER, SOCKET_FILE);
};

export { socketPathOf };
