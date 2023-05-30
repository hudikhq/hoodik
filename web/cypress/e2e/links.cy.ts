describe('Handle directories', () => {
  it('can create dir', () => {
    const email = `test+${Math.random() * 100000}@test.com`
    const password = `some-very-strong-password-${Math.random() * 123123123}`

    cy.createUser(email, password)

    cy.get('input[name="upload-file-input"]')
      .selectFile('./public/android-chrome-512x512.png', {
        force: true
      })
      .wait(5000)

    cy.get('button[name="actions-dropdown"]').click()
    cy.get('button[name="public-link"]').click()

    cy.get('button').contains('span', 'Create link').click()

    cy.get('input[name="link"]')
      .invoke('val')
      .then((val) => {
        cy.visit(val as string).wait(5000)
        cy.get('img[name=original]').should('have.attr', 'alt', 'android-chrome-512x512.png')
      })
  })
})
