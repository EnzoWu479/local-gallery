// Only one media element plays at a time: whoever starts playing claims the
// player, and the previous one is paused.
class PlayerState {
  #current: HTMLMediaElement | null = null;

  claim(element: HTMLMediaElement) {
    if (this.#current && this.#current !== element) this.#current.pause();
    this.#current = element;
  }
}

export const player = new PlayerState();
