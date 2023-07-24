describe('Handle directories', () => {
  it('can create dir', () => {
    const email = `test+${Math.random() * 100000}@test.com`
    const password = `some-very-strong-password-${Math.random() * 123123123}`

    cy.createUser(email, password)
    cy.get('button[name=create-dir]').click()

    cy.get('input[id=name]').type('Test_dir')
    cy.get('button').contains('Create').click()

    cy.contains('Test_dir')

    cy.get('button').contains('span', 'Test_dir').click().click()

    cy.get('button').contains('span', 'Test_dir').should('not.exist')

    cy.get('a').contains('span', 'My Files').click()
    cy.contains('span', 'Test_dir')

    cy.get('button').contains('span', 'Test_dir')

    cy.get('input[name="upload-file-input"]')
      .selectFile('./public/android-chrome-512x512.png', {
        force: true
      })
      .wait(5000)

    cy.get('img[name=thumbnail]').should('have.attr', 'alt', 'android-chrome-512x512.png')
    cy.get('button').contains('span', 'android-chrome-512x512.png').click().click()

    cy.get('img[name=original]').should('have.attr', 'alt', 'android-chrome-512x512.png')
    cy.get('button[name="preview-details"]').click()

    cy.contains('div', 'image/png')
  })
})
