import * as otp from 'cypress-otp'

function getEmail() {
  return `test+${Math.random() * 100000}@test.com`
}

let email = null
let password = null
let secret = null
let privateKey = null

describe('Registration to the application', () => {
  it('can register', () => {
    cy.viewport(1920, 1080)
    cy.visit(`${Cypress.env('APP_CLIENT_URL')}/auth/register`)

    email = getEmail()
    password = `some-very-strong-password-${Math.random() * 123123123}`

    cy.contains('Your email')
    cy.get('input[id=email]').type(email)
    cy.get('input[id=password]').type(password)
    cy.get('input[id=confirm_password]').type(password)
    cy.get('button').contains('Next').click().url().should('include', 'register/key')

    cy.get('textarea[id=unencrypted_private_key]')
      .invoke('val')
      .then(async (val) => {
        privateKey = val
        expect(privateKey).to.contain('BEGIN RSA PRIVATE KEY')
      })

    cy.get('input[id=i_have_stored_my_private_key]').check()
    cy.get('button').contains('Next').click().url().should('include', 'register/two-factor')

    cy.get('input[id=secret]')
      .invoke('val')
      .then(async (val) => {
        secret = val
        console.log(val)
        const token = otp(val)

        cy.get('input[id=token]').type(token)
        cy.get('button').contains('Register with Two Factor').click()
      })

    cy.get('a').contains('Logout').click().url().should('include', 'auth/login')
  })
})

describe('Login to the application', () => {
  it('can login with email and password with OPT token', () => {
    cy.viewport(1920, 1080)
    cy.visit(`${Cypress.env('APP_CLIENT_URL')}/auth/login`)

    cy.get('input[id=email]').type(email)
    cy.get('input[id=password]').type(password)
    cy.get('input[id=token]').type(otp(secret))
    cy.get('button').contains('Login').click()
  })
  it('can login with private key', () => {
    cy.viewport(1920, 1080)
    cy.visit(`${Cypress.env('APP_CLIENT_URL')}/auth/login`).wait(2000)

    cy.get('a').contains('Login With Private Key').click().url().should('include', 'private-key')
    cy.get('textarea[id=privateKey]').type(privateKey, { delay: 1 })
    cy.get('button').contains('Login').click()
  })
})
