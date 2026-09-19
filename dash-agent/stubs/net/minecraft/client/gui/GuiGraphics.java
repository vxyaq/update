package net.minecraft.client.gui;

import net.minecraft.resources.ResourceLocation;

/** Stub wyłącznie do kompilacji agenta. */
public class GuiGraphics {
    public void blit(ResourceLocation loc, int x, int y, int blitOffset,
            float u, float v, int w, int h, int texW, int texH) { }
    public void fill(int minX, int minY, int maxX, int maxY, int color) { }
    public int drawString(Font font, String text, int x, int y, int color) { return 0; }
}
