import AbstractError from '../../errors/AbstractError.js';

/**
 * A prompt was reached on a path that cannot receive an answer.
 *
 * This is a programming error rather than an operator error: a code path that
 * asks a question has to be gated on interactivity before it gets here. It
 * exists so the mistake surfaces as a reported failure instead of a process
 * that waits forever, or drains its event loop and exits successfully with
 * nothing done.
 */
export default class NonInteractivePromptError extends AbstractError {
  /**
   * @param {string} [question]
   */
  constructor(question) {
    super(`Tried to ask "${question ?? 'a question'}" without a terminal to answer it`);

    this.question = question;
  }
}
