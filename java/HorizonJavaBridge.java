// SPDX-License-Identifier: AGPL-3.0-or-later

import java.io.*;
import java.nio.file.*;
import java.net.*;
import java.util.*;
import java.util.jar.*;
import java.lang.reflect.*;
/** Experimental Java JAR lifecycle bridge, NOT a Bukkit or Paper API implementation. */
class HorizonJavaBridge {
 record Plugin(URLClassLoader loader, Object instance, Method disable, String name) {}
 public static void main(String[] args)throws Exception{
  List<Plugin> plugins=new ArrayList<>();
  try(var files=Files.list(Path.of(args[0]))){
   for(Path path:files.filter(f->f.toString().endsWith(".jar")).sorted().toList()){
    try(var jar=new JarFile(path.toFile())){
     var entry=jar.getJarEntry("plugin.yml");
     if(entry==null){System.out.println("SKIP (no plugin.yml): "+path);continue;}
     Map<String,String> meta=new HashMap<>();
     try(var lines=new BufferedReader(new InputStreamReader(jar.getInputStream(entry)))){
      for(String line;(line=lines.readLine())!=null;){
       var kv=line.split(":",2);
       if(kv.length==2&&!line.startsWith(" "))meta.put(kv[0].trim(),kv[1].trim());
      }
     }
     String main=meta.get("main");if(main==null){System.out.println("SKIP (no main): "+path);continue;}
     URLClassLoader loader=new URLClassLoader(new URL[]{path.toUri().toURL()},HorizonJavaBridge.class.getClassLoader());
     try{
      Class<?> cls=Class.forName(main,true,loader);
      Object instance=cls.getConstructor().newInstance();
      Method enable=cls.getMethod("onEnable");Method disable=cls.getMethod("onDisable");
      enable.invoke(instance);
      plugins.add(new Plugin(loader,instance,disable,meta.getOrDefault("name",main)));
      System.out.println("ENABLED "+main);
     }catch(Throwable e){System.out.println("FAILED "+main+": "+e);loader.close();}
    }catch(Exception e){System.out.println("INVALID JAR "+path+": "+e);}
   }
  }
  System.out.flush();
  try(var stdin=new BufferedReader(new InputStreamReader(System.in))){
   for(String line;(line=stdin.readLine())!=null;){
    if(line.equals("STOP"))break;
    if(line.startsWith("COMMAND\t"))System.out.println("COMMAND "+line.substring(8));
    System.out.flush();
   }
  }finally{
   for(int i=plugins.size()-1;i>=0;i--){
    Plugin p=plugins.get(i);
    try{p.disable.invoke(p.instance);System.out.println("DISABLED "+p.name);}catch(Throwable e){System.out.println("DISABLE ERROR "+p.name);}
    p.loader.close();
   }
  }
 }
}
