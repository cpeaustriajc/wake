interface Success<Value> {
  readonly ok: true;
  readonly value: Value;
}

interface Failure {
  readonly ok: false;
  readonly problem: string;
}

type Outcome<Value> = Failure | Success<Value>;

const success = <Value>(value: Value): Success<Value> => ({ ok: true, value });

const failure = (problem: string): Failure => ({ ok: false, problem });

export { failure, success };
export type { Failure, Outcome, Success };
