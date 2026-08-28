import AbstractError from '../../errors/AbstractError.js';

/**
 * The provider's API could not be reached, or answered with something that is
 * not a provider response at all.
 *
 * Raised at the request, where the transport failure is a fact rather than a
 * phrase. Recognising `fetch failed` in a message instead would let any text
 * carrying those words be read as this node's own network failing.
 */
export default class ProviderUnreachableError extends AbstractError {
}
