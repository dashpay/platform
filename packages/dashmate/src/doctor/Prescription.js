export const SEVERITY = {
  LOW: 1,
  MEDIUM: 2,
  HIGH: 3,
};

export class Prescription {
  /**
   * @type {Problem[]}
   */
  #orderedProblems;

  /**
   * @param {Problem[]} problems
   */
  constructor(problems) {
    const orderedProblems = [...problems];
    orderedProblems.sort((a, b) => b.getSeverity() - a.getSeverity());
    this.#orderedProblems = orderedProblems;
  }

  /**
   * @return {number} - Severity level
   */
  getSeverity() {
    return this.#orderedProblems
      .reduce((severity, problem) => (
        Math.max(severity, problem.getSeverity())
      ), SEVERITY.LOW);
  }

  /**
   * Get problems ordered by severity level
   *
   * @return {Problem[]}
   */
  getOrderedProblems() {
    return this.#orderedProblems;
  }
}
