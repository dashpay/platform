/**
 * Every documentation link dashmate prints, in one place.
 *
 * Gathered because they were repeated inline: the SSL-certificates anchor
 * appeared three times in one file alone, and the certificate troubleshooting
 * article was a constant in another. A link that lives in several places is one
 * that gets updated in some of them - and dashmate has already shipped links
 * that answered 404, which costs the command the credibility it needs at the
 * moment an operator is following its instructions.
 *
 * Two forms appear here, deliberately. `docs.dash.org/<slug>` is a redirect
 * configured in the documentation site's dashboard; those exist for some pages
 * and not others. Where no redirect was created, the published path is used
 * instead - it resolves today, and a link that resolves beats a shorter one
 * that does not.
 */
export const DOCS_LINKS = {
  /** Choosing and configuring a certificate provider. */
  SSL_CERTIFICATES: 'https://docs.dash.org/en/stable/docs/user/masternodes/setup-evonode.html#ssl-certificates',

  /** Why renewal fails, and what to do about each cause. */
  CERTIFICATE_TROUBLESHOOTING: 'https://docs.dash.org/en/stable/docs/user/masternodes/troubleshooting-certificates.html',

  /** Registering an evonode's collateral from Dash Core. */
  EVONODE_COLLATERAL: 'https://docs.dash.org/evonode-setup-core-collateral',

  /** Registering a masternode's collateral from Dash Core. */
  MASTERNODE_COLLATERAL: 'https://docs.dash.org/mn-setup-core-collateral',

  /** Dash Masternode Tool. */
  DMT_SETUP: 'https://docs.dash.org/dmt-setup',
};

export default DOCS_LINKS;
