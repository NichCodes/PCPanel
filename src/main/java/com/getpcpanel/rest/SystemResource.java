package com.getpcpanel.rest;

import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;

import com.getpcpanel.rest.model.dto.OnboardingDto;
import com.getpcpanel.util.StartupOnboarding;

import io.quarkus.runtime.Quarkus;
import lombok.extern.log4j.Log4j2;

/**
 * Application lifecycle control exposed to the web UI. The UI (the Tauri desktop window) is the one
 * place every platform can quit the otherwise-headless backend from without resorting to a terminal.
 */
@Log4j2
@Path("/api/system")
@ApplicationScoped
@Produces(MediaType.APPLICATION_JSON)
public class SystemResource {
    @Inject StartupOnboarding onboarding;

    /** Flipped by {@link #quit()} and observed by the Tauri shell via {@link #quitting()} so it can tear
     *  its window/Dock/tray down even when the backend's own exit is delayed or (in dev) never stops the
     *  HTTP server. Single instance ({@code @ApplicationScoped}), so a plain volatile field is enough. */
    private volatile boolean quitting;

    /**
     * One-time onboarding hint for the UI (which welcome/update dialog to show, the version, and the
     * changelog link). The UI fetches this once on load.
     */
    @GET
    @Path("/onboarding")
    public OnboardingDto onboarding() {
        return onboarding.info();
    }

    /** Mark the onboarding dialog as shown so it does not reappear on refresh. */
    @POST
    @Path("/onboarding/ack")
    public Response acknowledgeOnboarding() {
        onboarding.acknowledge();
        return Response.noContent().build();
    }

    /**
     * Shuts the application down. Sets the {@link #quitting} flag (so the desktop shell can react
     * immediately) and uses {@link Quarkus#asyncExit()} so this request can return its response before
     * the JVM stops; the normal Quarkus shutdown sequence then runs (ShutdownEvent → device handlers
     * close, tray icon is removed, AppShutdownState flips, pending saves flush), exactly as a tray "Exit"
     * would. The UI shows a "stopped" state afterwards since the server is then gone.
     */
    @POST
    @Path("/quit")
    public Response quit() {
        log.info("Quit requested from the web UI; shutting down");
        quitting = true;
        Quarkus.asyncExit(0);
        return Response.accepted().build();
    }

    /**
     * Whether a shutdown has been requested. The Tauri desktop shell polls this so it can close its
     * window and exit even when it is not the process that spawned the backend (dev mode) — there it
     * has no child process to watch. Serialised as a bare JSON boolean.
     */
    @GET
    @Path("/quitting")
    public boolean quitting() {
        return quitting;
    }
}
