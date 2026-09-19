package com.dash.agent;

import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassWriter;

/** ClassWriter który nie ładuje klas gry przy liczeniu ramek (bezpieczny w agencie). */
public class DashClassWriter extends ClassWriter {
    public DashClassWriter(ClassReader reader, int flags) {
        super(reader, flags);
    }

    @Override
    protected String getCommonSuperClass(String type1, String type2) {
        return "java/lang/Object";
    }
}
