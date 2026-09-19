package com.dash.agent;

import java.io.InputStream;

import net.minecraft.client.Minecraft;
import net.minecraft.client.Options;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.Renderable;
import net.minecraft.client.gui.screens.JoinMultiplayerScreen;
import net.minecraft.client.gui.screens.LanguageSelectScreen;
import net.minecraft.client.gui.screens.OptionsScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.SelectWorldScreen;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.client.renderer.texture.TextureManager;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.ResourceLocation;

import com.mojang.blaze3d.platform.NativeImage;

/**
 * DashMenu — ekran startowy w stylu Lunar (panorama + logo + przyciski),
 * napisany pod silnik 1.20.5+ (Mojmap). Layout na bazie
 * Hobbyshop/Lunar-Client-Main-Menu (MIT, patrz NOTICE).
 */
public class DashMenu extends Screen {

    private ResourceLocation bg;
    private ResourceLocation logo;
    private boolean assetsOk;

    public DashMenu() {
        super(Component.literal("Dash Menu"));
    }

    @Override
    protected void init() {
        registerAssets();
        int cx = this.width / 2;
        int y = this.height / 2;
        this.addRenderableWidget(Button.builder(Component.literal("SINGLEPLAYER"),
                new Button.OnPress() {
                    @Override public void onPress(Button b) {
                        DashMenu.this.minecraft.setScreen(new SelectWorldScreen(DashMenu.this));
                    }
                }).bounds(cx - 100, y, 200, 20).build());
        this.addRenderableWidget(Button.builder(Component.literal("MULTIPLAYER"),
                new Button.OnPress() {
                    @Override public void onPress(Button b) {
                        DashMenu.this.minecraft.setScreen(new JoinMultiplayerScreen(DashMenu.this));
                    }
                }).bounds(cx - 100, y + 24, 200, 20).build());
        this.addRenderableWidget(Button.builder(Component.literal("OPTIONS"),
                new Button.OnPress() {
                    @Override public void onPress(Button b) {
                        Options opts = DashMenu.this.minecraft.options;
                        DashMenu.this.minecraft.setScreen(new OptionsScreen(DashMenu.this, opts));
                    }
                }).bounds(cx - 100, y + 48, 100, 20).build());
        this.addRenderableWidget(Button.builder(Component.literal("QUIT"),
                new Button.OnPress() {
                    @Override public void onPress(Button b) {
                        DashMenu.this.minecraft.stop();
                    }
                }).bounds(cx, y + 48, 100, 20).build());
        this.addRenderableWidget(Button.builder(Component.literal("L"),
                new Button.OnPress() {
                    @Override public void onPress(Button b) {
                        Minecraft mc = DashMenu.this.minecraft;
                        mc.setScreen(new LanguageSelectScreen(DashMenu.this, mc.options,
                                mc.getLanguageManager()));
                    }
                }).bounds(this.width - 28, this.height - 28, 20, 20).build());
    }

    private void registerAssets() {
        if (this.bg != null) {
            return;
        }
        TextureManager tm = this.minecraft.getTextureManager();
        this.bg = registerPng(tm, "dash", "panorama/panorama_0.png");
        this.logo = registerPng(tm, "dash", "logo.png");
        this.assetsOk = this.bg != null && this.logo != null;
    }

    private static ResourceLocation registerPng(TextureManager tm, String ns, String path) {
        InputStream in = null;
        try {
            in = DashMenu.class.getResourceAsStream("/assets/" + ns + "/" + path);
            if (in == null) {
                return null;
            }
            NativeImage img = NativeImage.read(in);
            ResourceLocation loc = new ResourceLocation(ns, path);
            tm.register(loc, new DynamicTexture(img));
            return loc;
        } catch (Throwable t) {
            System.out.println("[dash] tekstura pominięta (" + path + "): " + t);
            return null;
        } finally {
            if (in != null) {
                try { in.close(); } catch (Throwable ignored) { }
            }
        }
    }

    @Override
    public void render(GuiGraphics g, int mouseX, int mouseY, float partial) {
        try {
            renderDash(g, partial);
        } catch (Throwable t) {
            t.printStackTrace();
            // Fallback: nigdy czarny ekran — waniliowe tło + komunikat.
            g.fill(0, 0, this.width, this.height, 0xFF101010);
            g.drawString(this.minecraft.font, "Dash menu error - vanilla fallback", 5, 5, 0xFF5555);
            return;
        }
        for (int i = 0; i < this.renderables.size(); i++) {
            Renderable r = this.renderables.get(i);
            r.render(g, mouseX, mouseY, partial);
        }
    }

    private void renderDash(GuiGraphics g, float partial) {
        // Tło: panorama z wolnym przesunięciem + przyciemnienie.
        if (this.bg != null) {
            long t = System.currentTimeMillis() / 50L;
            float u = (t % 2400L) / 2400.0f * 64.0f;
            g.blit(this.bg, 0, 0, 0, u, 0.0f, this.width, this.height, 256, 256);
        } else {
            g.fill(0, 0, this.width, this.height, 0xFF0B0E14);
        }
        g.fill(0, 0, this.width, this.height, 0x80000000);

        int cx = this.width / 2;
        int cy = this.height / 2;
        if (this.logo != null) {
            g.blit(this.logo, cx - 24, cy - 78, 0, 0.0f, 0.0f, 48, 48, 48, 48);
        }
        String title = "DASH CLIENT";
        g.drawString(this.minecraft.font, title,
                cx - this.minecraft.font.width(title) / 2, cy - 24, 0xFFFFFFFF);
        String ver = "Dash Client " + DashAgent.mcVersion;
        g.drawString(this.minecraft.font, ver, 7, this.height - 12, 0x66FFFFFF);
        String copy = "Copyright Mojang Studios. Do not distribute!";
        g.drawString(this.minecraft.font, copy,
                this.width - this.minecraft.font.width(copy) - 7, this.height - 12, 0x66FFFFFF);
    }
}
