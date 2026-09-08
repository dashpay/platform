import AbstractError from '../../errors/AbstractError.js';

/**
 * Another command held or took the configuration lock, so renewal stopped part
 * way through.
 *
 * Typed rather than recognised by its wording. The certificate authority quotes
 * the responder's own page back into its problem detail, so a machine answering
 * on port 80 could otherwise put this phrase in front of dashmate and have an
 * operator told to stop and wait for a command that was never running.
 */
export default class ConfigurationLockLostError extends AbstractError {
}
