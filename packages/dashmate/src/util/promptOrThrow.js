import NonInteractivePromptError from './errors/NonInteractivePromptError.js';

/**
 * Ask the operator a question, or fail loudly when there is nobody to ask.
 *
 * Every prompt goes through here. Neither listr2 nor enquirer refuses to build
 * a prompt on a stream that cannot answer - listr2 has no terminal check at all
 * and enquirer's guard does not fire on the default stdin - so a prompt reached
 * unattended waits for the writer's lifetime and then leaves the process to
 * exit with nothing done. Refusing up front turns that silence into an error
 * with a name.
 *
 * Interactivity must be stated positively. A guard phrased the other way round
 * lets any caller that forgets to pass it enable prompting by omission, and one
 * of those callers renews certificates unattended inside a container.
 *
 * @param {Object} task - the listr2 task the prompt is rendered by
 * @param {Object} options - enquirer prompt options
 * @param {Object} context
 * @param {boolean} [context.interactive]
 * @return {Promise<*>}
 */
export default function promptOrThrow(task, options, { interactive } = {}) {
  if (interactive !== true) {
    throw new NonInteractivePromptError(options?.message);
  }

  return task.prompt(options);
}
