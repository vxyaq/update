package net.minecraft.client.gui.screens;

import java.util.ArrayList;
import java.util.List;

import net.minecraft.client.Minecraft;
import net.minecraft.network.chat.Component;
import net.minecraft.client.gui.components.Renderable;

/** Stub wyłącznie do kompilacji agenta. */
public class Screen {
    protected Minecraft minecraft;
    protected int width;
    protected int height;
    protected List<Renderable> renderables = new ArrayList<Renderable>();

    protected Screen(Component title) { }

    protected void init() { }

    public void render(net.minecraft.client.gui.GuiGraphics g, int mouseX, int mouseY, float partial) { }

    protected <T extends Renderable> T addRenderableWidget(T widget) { return widget; }
}
