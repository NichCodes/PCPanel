import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { OnboardingService } from './services/onboarding.service';
import { DebugService } from './services/debug.service';
import { IconComponent, ModalComponent } from './ui';

const GITHUB_URL = 'https://github.com/nvdweem/PCPanel';

/**
 * Shows the first-run welcome dialog or the post-install/update dialog once on startup, based on the
 * backend onboarding hint. Both explain that the app keeps running in the tray. Hosted at the app root
 * so it overlays regardless of route.
 */
@Component({
  selector: 'app-onboarding',
  standalone: true,
  imports: [ModalComponent, IconComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <!-- First run -->
    <pc-modal [open]="view() === 'new-user'" heading="Welcome to PCPanel" [width]="540" (dismiss)="dismiss()">
      <div class="ob">
        <p class="lead">Thanks for installing PCPanel — third-party control software for your PCPanel device.</p>
        <p class="body">Plug your device in over USB and it's detected automatically. Click a knob, slider or button on
          the device view to assign what it controls (app/device volume, mute, media keys, shortcuts, OBS, and more).</p>
        <a class="link" [href]="githubUrl" target="_blank" rel="noopener noreferrer">
          <pc-icon name="external-link" [size]="13"></pc-icon> Setup &amp; usage instructions on GitHub
        </a>
        <div class="tray-note">{{ trayNote }}</div>
        <div class="actions"><button class="pc-btn primary" (click)="dismiss()">Get started</button></div>
      </div>
    </pc-modal>

    <!-- After an installer update -->
    <pc-modal [open]="view() === 'post-install'" heading="PCPanel is up to date" [width]="540" (dismiss)="dismiss()">
      <div class="ob">
        <p class="lead">PCPanel has been updated{{ version() ? ' to ' + version() : '' }} and is running again.</p>
        @if (changelogUrl()) {
          <a class="link" [href]="changelogUrl()" target="_blank" rel="noopener noreferrer">
            <pc-icon name="external-link" [size]="13"></pc-icon> What's new in this version
          </a>
        }
        <div class="tray-note">{{ trayNote }}</div>
        <div class="actions"><button class="pc-btn primary" (click)="dismiss()">Done</button></div>
      </div>
    </pc-modal>
  `,
  styles: [`
    .ob { width: 480px; max-width: 100%; display: flex; flex-direction: column; gap: 14px; }
    .lead { font-size: 14px; color: var(--text-1); margin: 0; line-height: 1.5; }
    .body { font-size: 12.5px; color: var(--text-2); margin: 0; line-height: 1.6; }
    .link { display: inline-flex; align-items: center; gap: 6px; font-size: 13px; color: var(--accent); text-decoration: none; width: fit-content; }
    .link:hover { text-decoration: underline; }
    .tray-note { font-size: 12px; color: var(--text-2); line-height: 1.55; background: var(--panel); border: 1px solid var(--line); border-radius: var(--r-md); padding: 11px 13px; }
    .actions { display: flex; justify-content: flex-end; margin-top: 4px; }
  `],
})
export class OnboardingComponent {
  private readonly onboarding = inject(OnboardingService);
  private readonly debug = inject(DebugService);

  readonly githubUrl = GITHUB_URL;
  private readonly dismissed = signal(false);
  private readonly info = this.onboarding.info;

  readonly version = computed(() => this.info()?.version ?? '');
  readonly changelogUrl = computed(() => this.info()?.changelogUrl ?? '');

  /** Which dialog to show, or null. A Debug-page preview wins; otherwise the backend onboarding intent
   *  (until dismissed). */
  readonly view = computed<'new-user' | 'post-install' | null>(() => {
    const preview = this.debug.onboardingPreview();
    if (preview === 'new-user' || preview === 'post-install') return preview;
    if (this.dismissed()) return null;
    const intent = this.info()?.intent;
    return intent === 'new-user' || intent === 'post-install' ? intent : null;
  });

  readonly trayNote = 'PCPanel keeps running in your system tray — click its icon any time to reopen this window.';

  constructor() {
    this.onboarding.load();
  }

  dismiss(): void {
    // A Debug-page preview just closes (don't acknowledge the real backend intent).
    if (this.debug.onboardingPreview()) {
      this.debug.previewOnboarding('');
      return;
    }
    this.dismissed.set(true);
    this.onboarding.ack();
  }
}
