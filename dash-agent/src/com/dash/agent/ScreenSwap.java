package com.dash.agent;

import java.lang.instrument.ClassFileTransformer;
import java.security.ProtectionDomain;

import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassVisitor;
import org.objectweb.asm.ClassWriter;
import org.objectweb.asm.Label;
import org.objectweb.asm.MethodVisitor;
import org.objectweb.asm.Opcodes;

/**
 * Podmienia TitleScreen na DashMenu w Minecraft.setScreen(Screen).
 * Działa na 1.20.5+ (sygnatura setScreen stabilna).
 */
public class ScreenSwap implements ClassFileTransformer, Opcodes {

    private static final String MINECRAFT = "net/minecraft/client/Minecraft";
    private static final String TITLE = "net/minecraft/client/gui/screens/TitleScreen";
    private static final String MENU = "com/dash/agent/DashMenu";
    private static final String SETSCREEN_DESC = "(Lnet/minecraft/client/gui/screens/Screen;)V";

    @Override
    public byte[] transform(ClassLoader loader, String className, Class<?> classBeingRedefined,
            ProtectionDomain protectionDomain, byte[] classBuffer) {
        if (!MINECRAFT.equals(className)) {
            return null;
        }
        try {
            ClassReader cr = new ClassReader(classBuffer);
            ClassWriter cw = new DashClassWriter(cr, ClassWriter.COMPUTE_FRAMES);
            cr.accept(new ClassVisitor(ASM9, cw) {
                @Override
                public MethodVisitor visitMethod(int access, String name, String desc,
                        String signature, String[] exceptions) {
                    MethodVisitor mv = super.visitMethod(access, name, desc, signature, exceptions);
                    if ("setScreen".equals(name) && SETSCREEN_DESC.equals(desc)) {
                        return new MethodVisitor(ASM9, mv) {
                            @Override
                            public void visitCode() {
                                super.visitCode();
                                mv.visitVarInsn(ALOAD, 1);
                                mv.visitTypeInsn(INSTANCEOF, TITLE);
                                Label skip = new Label();
                                mv.visitJumpInsn(IFEQ, skip);
                                mv.visitVarInsn(ALOAD, 0);
                                mv.visitTypeInsn(NEW, MENU);
                                mv.visitInsn(DUP);
                                mv.visitMethodInsn(INVOKESPECIAL, MENU, "<init>", "()V", false);
                                mv.visitVarInsn(ASTORE, 1);
                                mv.visitLabel(skip);
                            }
                        };
                    }
                    return mv;
                }
            }, 0);
            return cw.toByteArray();
        } catch (Throwable t) {
            System.out.println("[dash] transform pominięty: " + t);
            return null;
        }
    }
}
