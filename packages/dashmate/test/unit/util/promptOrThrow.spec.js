import promptOrThrow from '../../../src/util/promptOrThrow.js';
import NonInteractivePromptError from '../../../src/util/errors/NonInteractivePromptError.js';

describe('promptOrThrow', () => {
  it('should ask the operator when a human is there to answer', async function it() {
    const task = { prompt: this.sinon.stub().resolves('yes') };

    const answer = await promptOrThrow(task, { message: 'Continue?' }, { interactive: true });

    expect(answer).to.equal('yes');
    expect(task.prompt).to.have.been.calledOnceWithExactly({ message: 'Continue?' });
  });

  // A prompt built on a stream nobody is reading never throws and never
  // settles: listr2 has no TTY check and enquirer's guard does not fire on the
  // default stdin, so the process drains its event loop and exits 0 with
  // nothing done. Refusing to construct the prompt is what turns that silence
  // into a reported failure.
  it('should refuse to prompt when nobody can answer', function it() {
    const task = { prompt: this.sinon.stub() };

    expect(() => promptOrThrow(task, { message: 'Continue?' }, { interactive: false }))
      .to.throw(NonInteractivePromptError);

    expect(task.prompt).to.not.have.been.called();
  });

  // Prompting has to be opted into positively. A guard phrased as "prompt
  // unless told otherwise" lets any caller that forgets the flag - the helper's
  // unattended renewal, for one - enable prompting by omission.
  it('should refuse to prompt when interactivity was never stated', function it() {
    const task = { prompt: this.sinon.stub() };

    expect(() => promptOrThrow(task, {}, {})).to.throw(NonInteractivePromptError);
    expect(() => promptOrThrow(task, {}, { interactive: 'yes' })).to.throw(NonInteractivePromptError);
    expect(() => promptOrThrow(task, {}, { interactive: 1 })).to.throw(NonInteractivePromptError);

    expect(task.prompt).to.not.have.been.called();
  });

  it('should name the question it refused to ask', function it() {
    const task = { prompt: this.sinon.stub() };

    expect(() => promptOrThrow(task, { message: 'Switch to Let\'s Encrypt?' }, {}))
      .to.throw('Switch to Let\'s Encrypt?');
  });
});
