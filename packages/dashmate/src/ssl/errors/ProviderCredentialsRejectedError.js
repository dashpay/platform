import AbstractError from '../../errors/AbstractError.js';

/**
 * This node's provider credentials will not be accepted.
 *
 * Raised where the key is examined rather than recognised by its wording. A
 * key that is absent, empty or malformed never reaches the provider, so there
 * is no numeric code to classify it by - and without a type it fell through to
 * "could not work out why", which sends an operator to support for something
 * they can repair in one command.
 */
export default class ProviderCredentialsRejectedError extends AbstractError {
}
