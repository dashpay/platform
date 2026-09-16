import AbstractError from '../../errors/AbstractError.js';

/**
 * The provider's own preflight could not confirm this node answers on port 80.
 *
 * Raised where the check runs, not inferred from the text it produced. Which of
 * the two readings applies is still unknown - nothing replied, or something
 * replied wrongly - and this says only what was observed.
 */
export default class VerificationServerUnreachableError extends AbstractError {
}
