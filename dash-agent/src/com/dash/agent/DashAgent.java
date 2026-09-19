package com.dash.agent;

import java.lang.instrument.Instrumentation;

/**
 * DashAgent — podpina moduły .java (DashMenu i kolejne) do gry przez -javaagent.
 * Uruchamiane PRZED main Minecraft, podmienia TitleScreen na DashMenu.
 */
public class DashAgent {

    /** Wersja MC przekazana jako argument agenta (-javaagent:jar=1.21.11). */
    public static volatile String mcVersion = "";

    public static void premain(String args, Instrumentation inst) {
        if (args != null && !args.trim().isEmpty()) {
            mcVersion = args.trim();
        }
        inst.addTransformer(new ScreenSwap(), false);
        System.out.println("[dash] agent zaladowany (mc=" + mcVersion + ")");
    }
}
