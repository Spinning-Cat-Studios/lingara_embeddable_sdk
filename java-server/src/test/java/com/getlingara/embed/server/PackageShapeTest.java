package com.getlingara.embed.server;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

import java.io.InputStream;
import java.lang.module.ModuleDescriptor;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Set;
import java.util.TreeSet;
import javax.xml.parsers.DocumentBuilderFactory;
import org.junit.jupiter.api.Test;
import org.w3c.dom.Document;
import org.w3c.dom.Element;
import org.w3c.dom.NodeList;

class PackageShapeTest {
  /**
   * 1.10.26af AC25: the generated POM's only dependency is lingara-java, no lingara-embed-java
   * (player) artifact appears, and module-info requires only the library's module (java.base is
   * mandated).
   */
  @Test
  void onlyTheLibraryIsRequired() throws Exception {
    Path pomFile = Path.of(System.getProperty("lingara.pom"));
    DocumentBuilderFactory factory = DocumentBuilderFactory.newInstance();
    factory.setFeature("http://apache.org/xml/features/disallow-doctype-decl", true);
    Document pom = factory.newDocumentBuilder().parse(pomFile.toFile());
    NodeList deps = pom.getElementsByTagName("dependency");
    List<String> coordinates = new ArrayList<>();
    for (int i = 0; i < deps.getLength(); i++) {
      Element d = (Element) deps.item(i);
      coordinates.add(text(d, "groupId") + ":" + text(d, "artifactId") + ":" + text(d, "scope"));
    }
    assertEquals(List.of("com.getlingara:lingara-java:compile"), coordinates);
    assertEquals("lingara-embed-server-java", text(pom.getDocumentElement(), "artifactId"));
    assertFalse(Files.readString(pomFile).contains("lingara-embed-java"));

    ModuleDescriptor module;
    try (InputStream in = Files.newInputStream(Path.of(System.getProperty("lingara.moduleInfo")))) {
      module = ModuleDescriptor.read(in);
    }
    assertEquals("com.getlingara.embed.server", module.name());
    Set<String> requires = new TreeSet<>();
    for (ModuleDescriptor.Requires r : module.requires()) {
      if (!r.modifiers().contains(ModuleDescriptor.Requires.Modifier.MANDATED)) {
        requires.add(r.name());
      }
    }
    assertEquals(Set.of("com.getlingara.client"), requires);
  }

  private static String text(Element parent, String tag) {
    NodeList nodes = parent.getElementsByTagName(tag);
    return nodes.getLength() == 0 ? "" : nodes.item(0).getTextContent().trim();
  }
}
