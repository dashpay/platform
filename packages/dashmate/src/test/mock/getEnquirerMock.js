/**
 * Stand in for the enquirer instance listr2 builds for a prompt.
 *
 * listr2 takes the instance from `injectWrapper.enquirer` when one is present,
 * so a test can answer a prompt without a terminal - and, more importantly, can
 * assert that a prompt was never constructed at all on paths that must not ask.
 *
 * @param {Object} sinon
 * @param {...*} answers - one per prompt, in order
 * @return {{on: Function, prompt: Function, options: Object[]}}
 */
export default function getEnquirerMock(sinon, ...answers) {
  const options = [];
  const remaining = [...answers];

  return {
    options,
    on: sinon.stub(),
    prompt: sinon.stub().callsFake(async (promptOptions) => {
      options.push(...[].concat(promptOptions));

      return { default: remaining.length > 0 ? remaining.shift() : undefined };
    }),
  };
}
