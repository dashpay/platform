import fs from 'fs';
import dots from 'dot';

/**
 * @return {renderTemplate}
 */
export function renderTemplateFactory() {
  /**
   * Render template for a service
   *
   * @typedef {renderTemplate}
   * @param {string} templatePath
   * @param {object} variables
   *
   * @return {Object<string,string>}
   */
  function renderTemplate(templatePath, variables) {
    const templateString = fs.readFileSync(templatePath, 'utf-8');
    const template = dots.template(templateString);

    return template(variables);
  }

  return renderTemplate;
}
