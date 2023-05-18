/// <reference types="cypress" />
// ***********************************************
// This example commands.ts shows you how to
// create various custom commands and overwrite
// existing commands.
//
// For more comprehensive examples of custom
// commands please read more here:
// https://on.cypress.io/custom-commands
// ***********************************************
//
//
// -- This is a parent command --
// Cypress.Commands.add('login', (email, password) => { ... })
//
//
// -- This is a child command --
// Cypress.Commands.add('drag', { prevSubject: 'element'}, (subject, options) => { ... })
//
//
// -- This is a dual command --
// Cypress.Commands.add('dismiss', { prevSubject: 'optional'}, (subject, options) => { ... })
//
//
// -- This will overwrite an existing command --
// Cypress.Commands.overwrite('visit', (originalFn, url, options) => { ... })
//
// declare global {
//   namespace Cypress {
//     interface Chainable {
//       login(email: string, password: string): Chainable<void>
//       drag(subject: string, options?: Partial<TypeOptions>): Chainable<Element>
//       dismiss(subject: string, options?: Partial<TypeOptions>): Chainable<Element>
//       visit(originalFn: CommandOriginalFn, url: string, options: Partial<VisitOptions>): Chainable<Element>
//     }
//   }
// }

export {}
declare global {
  namespace Cypress {
    interface Chainable {
      createUser(email: string, password: string): Chainable<{ email: string; password: string }>
    }
  }
}

Cypress.Commands.add('createUser', (email: string, password: string) => {
  cy.viewport(1920, 1080)
  cy.visit(`${Cypress.env('APP_CLIENT_URL')}/auth/register`)

  cy.contains('Your email')
  cy.get('input[id=email]').type(email)
  cy.get('input[id=password]').type(password)
  cy.get('input[id=confirm_password]').type(password)
  cy.get('button').contains('Next').click().url().should('include', 'register/key')

  cy.get('input[id=i_have_stored_my_private_key]').check()
  cy.get('button').contains('Next').click().url().should('include', 'register/two-factor')
  cy.get('button').contains('Skip').click()

  return cy.wrap({
    email,
    password
  })
})
