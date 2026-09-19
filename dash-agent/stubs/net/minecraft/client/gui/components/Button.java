package net.minecraft.client.gui.components;

import net.minecraft.network.chat.Component;

/** Stub wyłącznie do kompilacji agenta. */
public class Button implements Renderable {
    public interface OnPress {
        void onPress(Button button);
    }

    public static class Builder {
        public Builder bounds(int x, int y, int w, int h) { return this; }
        public Button build() { return null; }
    }

    public static Builder builder(Component text, OnPress onPress) { return null; }

    @Override
    public void render(net.minecraft.client.gui.GuiGraphics g, int mouseX, int mouseY, float partial) { }
}
