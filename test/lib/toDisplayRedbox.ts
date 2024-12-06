import type { MatcherContext } from 'expect'
import { toMatchInlineSnapshot } from 'jest-snapshot'
import { assertHasRedbox, getRedboxCallStack } from './next-test-utils'
import type { BrowserInterface } from './browsers/base'

declare global {
  namespace jest {
    // eslint-disable-next-line @typescript-eslint/no-unused-vars -- module augmentation needs to match generic params even if unused
    interface Matchers<R> {
      /**
       * Inline snapshot matcher for a Redbox that's poppep up by default.
       * When a Redbox is hidden at first and requires manual display by clicking the toast,
       * use {@link toDisplayCollapsedRedbox} instead.
       * @param inlineSnapshot - The snapshot to compare against.
       */
      toDisplayRedbox(inlineSnapshot?: string): Promise<void>

      /**
       * Inline snapshot matcher for a Redbox that's collapsed by default.
       * When a Redbox is immediately displayed ,
       * use {@link toDisplayRedbox} instead.
       * @param inlineSnapshot - The snapshot to compare against.
       */
      toDisplayCollapsedRedbox(inlineSnapshot?: string): Promise<void>
    }
  }
}

expect.extend({
  async toDisplayRedbox(
    this: MatcherContext,
    browser: BrowserInterface,
    expectedRedboxSnapshot?: string
  ) {
    // Otherwise jest uses the async stack trace which makes it impossible to know the actual callsite of `toMatchSpeechInlineSnapshot`.
    // @ts-expect-error -- Not readonly
    this.error = new Error()
    // Abort test on first mismatch.
    // Subsequent actions will be based on an incorrect state otherwise and almost always fail as well.
    // TODO: Actually, we may want to proceed. Kinda nice to also do more assertions later.
    this.dontThrow = () => {}

    try {
      await assertHasRedbox(browser)
    } catch {
      // argument length is relevant.
      // Jest will update absent snapshots but fail if you specify a snapshot even if undefined.
      if (expectedRedboxSnapshot === undefined) {
        return toMatchInlineSnapshot.call(this, '<no redbox found>')
      } else {
        return toMatchInlineSnapshot.call(
          this,
          '<no redbox found>',
          expectedRedboxSnapshot
        )
      }
    }

    const redbox = {
      stack: await getRedboxCallStack(browser).catch(() => '<empty>'),
      // TODO: message, description, codeframe etc
    }

    // argument length is relevant.
    // Jest will update absent snapshots but fail if you specify a snapshot even if undefined.
    if (expectedRedboxSnapshot === undefined) {
      return toMatchInlineSnapshot.call(this, redbox)
    } else {
      return toMatchInlineSnapshot.call(this, redbox, expectedRedboxSnapshot)
    }
  },
  async toDisplayCollapsedRedbox(
    this: MatcherContext,
    browser: BrowserInterface,
    expectedRedboxSnapshot?: string
  ) {
    // Otherwise jest uses the async stack trace which makes it impossible to know the actual callsite of `toMatchSpeechInlineSnapshot`.
    // @ts-expect-error -- Not readonly
    this.error = new Error()
    // Abort test on first mismatch.
    // Subsequent actions will be based on an incorrect state otherwise and almost always fail as well.
    // TODO: Actually, we may want to proceed. Kinda nice to also do more assertions later.
    this.dontThrow = () => {}

    // TODO: Implement
    throw new Error('Not implemented')
  },
})
